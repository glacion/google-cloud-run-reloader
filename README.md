# Google Cloud Run Secret Reloader

A lightweight Cloud Run service that listens for Secret Manager version events and forces new revisions for any Cloud Run services still pointing at `latest` secret versions.
This keeps secrets in environment variables or mounted volumes fresh without manual redeploys.

## How it works
- Eventarc delivers Secret Manager `AddSecretVersion` audit log entries as CloudEvents to this service.
- The handler extracts the project and secret ID from the event payload.
- Cloud Run services in the project (all regions) are listed via the Cloud Run v2 API.
- Services that reference the updated secret using the `latest` version (env var `secretKeyRef` or secret volume) are selected.
- The service template revision is updated (metadata change only), which forces Cloud Run to create a new revision and pick up the new secret value.

## Requirements
- A Google Cloud project with billing enabled
- `gcloud` CLI authenticated to the target project
- Docker for building the container image (set `DOCKER_DEFAULT_PLATFORM=linux/amd64` on arm64 hosts)
- Rust toolchain (only if you plan to develop locally)

## Setup and deployment
Export commonly used values:

```bash
export PROJECT_ID=$(gcloud config get-value project)
export PROJECT_NUMBER=$(gcloud projects describe $PROJECT_ID --format='value(projectNumber)')
export REGION=us-central1
export SERVICE_ACCOUNT=reloader
# Needed if you're building on arm64 hosts
export DOCKER_DEFAULT_PLATFORM=linux/amd64
export DOCKER_BUILDKIT=1
```

Enable required services:

```bash
gcloud --project=$PROJECT_ID services enable \
  artifactregistry.googleapis.com \
  run.googleapis.com \
  eventarc.googleapis.com \
  secretmanager.googleapis.com
```

Turn on Secret Manager Data Access audit logs (required for Eventarc):
1. Open IAM & Admin → Audit Logs in the Cloud Console.
2. Search for **Secret Manager API**.
3. Enable all Data Access logs for the project and save.

Create the service account and grant permissions:

```bash
gcloud iam service-accounts create $SERVICE_ACCOUNT \
  --display-name="Reloader" \
  --project=$PROJECT_ID

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/run.developer"

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/artifactregistry.reader"

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/eventarc.eventReceiver"
```

Create (or ensure) an Artifact Registry Docker repository for the image:

```bash
gcloud artifacts repositories create gcr.io \
  --project=$PROJECT_ID \
  --location=us \
  --repository-format=docker
```

Build and push the container image:

```bash
docker build \
  --platform=linux/amd64 \
  --push \
  --tag=gcr.io/$PROJECT_ID/reloader:0.1.0 \
  .
```

Deploy to Cloud Run:

```bash
gcloud run deploy reloader \
  --concurrency=1 \
  --cpu=0.1 \
  --image=gcr.io/$PROJECT_ID/reloader:0.1.0 \
  --ingress=internal \
  --memory=128Mi \
  --min-instances=0 \
  --no-allow-unauthenticated \
  --project=$PROJECT_ID \
  --region=$REGION \
  --service-account=$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com

# Allow Eventarc to invoke the service
gcloud run services add-iam-policy-binding reloader \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --project=$PROJECT_ID \
  --region=$REGION \
  --role="roles/run.invoker"
```

Create the Eventarc trigger:

```bash
gcloud eventarc triggers create reloader \
  --destination-run-region=$REGION \
  --destination-run-service=reloader \
  --event-filters="methodName=google.cloud.secretmanager.v1.SecretManagerService.AddSecretVersion" \
  --event-filters="serviceName=secretmanager.googleapis.com" \
  --event-filters="type=google.cloud.audit.log.v1.written" \
  --location=global \
  --project=$PROJECT_ID \
  --service-account=$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com
```

## Verifying the setup
- Deploy a Cloud Run service that references a Secret Manager secret using the `latest` version (env var or secret volume).
- Add a new version to that secret (`gcloud secrets versions add SECRET --data-file=...`).
- Check Reloader logs (`gcloud run services logs read reloader --region $REGION`) and confirm the dependent service gets a new revision.

## Local development
Run locally with Application Default Credentials so the Cloud Run API can be called:

```bash
cargo run
```

The server listens on `0.0.0.0:8080` and expects CloudEvents. Example event payload for manual testing:

```json
{
  "protoPayload": {
    "authenticationInfo": { "principalEmail": "user@example.com" },
    "resourceName": "projects/123456789/secrets/my-secret/versions/5"
  }
}
```

You can POST this JSON as CloudEvent data using a tool like `cloudevents` CLI or by crafting CloudEvent headers; running inside Cloud Run via Eventarc requires no manual headers.

## Logging
Structured JSON logs are emitted via `tracing` with `RUST_LOG` respected (default `info`). Set `RUST_LOG=debug` during troubleshooting.
