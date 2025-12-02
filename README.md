# Google Cloud Run Reloader

Automatically trigger new revisions for Cloud Run services when their referenced Google Cloud Secrets are updated.

## Overview

When a secret in Google Cloud Secret Manager gets a new version, 
Cloud Run services referencing the ":latest" version of that secret via environment variables do not automatically pick up the new value until they are redeployed. 
Services mounting secrets as volumes will eventually see the change, but a redeploy ensures immediate consistency.

**Reloader** solves this by:
1.  Receiving CloudEvents (via Eventarc triggering on Secret Manager Audit Logs).
2.  identifying which Cloud Run services are using the updated secret.
3.  Forcing a new revision for those services by updating a specific annotation.

## How It Works

1.  **Receive Event:** The service listens on port `8080` for CloudEvents. It parses the `protoPayload` from the audit log entry.
2.  **Identify Secret:** It extracts the project ID and secret ID from the `resourceName` in the incoming event.
3.  **Scan Services:** It lists all Cloud Run services in the project (across all regions).
4.  **Filter:** It identifies services that:
    *   Reference the updated secret.
    *   Use the `latest` version of that secret (either in environment variables or volume mounts).
5.  **Trigger Update:** For every matching service, it updates the `reloader.glacion.com/timestamp` annotation in the service template with the current epoch timestamp. This metadata change forces Cloud Run to create a new revision.

## Prerequisites

*   **Google Cloud Project** with billing enabled.
*   **gcloud CLI** installed and configured.
*   **Rust** toolchain (for local development/building).

## Setup & Deployment

### 1. Environment Variables

Set the required environment variables for the setup commands:

```bash
export PROJECT_ID=$(gcloud config get-value project)
export PROJECT_NUMBER=$(gcloud projects describe $PROJECT_ID --format='value(projectNumber)')
export REGION=us-central1
export SERVICE_ACCOUNT=reloader
# Needed if you're running from arm machines
export DOCKER_DEFAULT_PLATFORM=linux/amd64
```

### 2. Enable APIs

Enable the necessary Google Cloud APIs:

```bash
gcloud services enable \
  run.googleapis.com \
  eventarc.googleapis.com \
  secretmanager.googleapis.com \
  cloudbuild.googleapis.com
```

### 3. Enable Audit Logs

Eventarc relies on Cloud Audit Logs to trigger on Secret Manager changes. You must enable Data Access audit logs for the Secret Manager API.

1.  Go to [IAM & Admin > Audit Logs](https://console.cloud.google.com/iam-admin/audit) in the Google Cloud Console.
2.  Search for "Secret Manager API".
3.  Select it and check **"Data Write"**.
4.  Click **Save**.

### 4. Create Service Account

Create a service account for the Reloader service and Eventarc trigger:

```bash
# Create service account
gcloud iam service-accounts create $SERVICE_ACCOUNT \
  --display-name="Reloader Service Account"

# Grant permission to list and update Cloud Run services
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/run.developer"

# Grant permission to resolve project numbers
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/resourcemanager.projectAccessor"

# Grant permission for Eventarc to receive events
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/eventarc.eventReceiver"

# Grant permission for Eventarc to invoke the Reloader service
gcloud run services add-iam-policy-binding $SERVICE_ACCOUNT \
  --region $REGION \
  --member="serviceAccount:$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/run.invoker"

# Allow Pub/Sub to create tokens for the service account (required for Eventarc)
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:service-$PROJECT_NUMBER@gcp-sa-pubsub.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountTokenCreator"
```

### 5. Build and Deploy

1.  **Build the container image:**

    ```bash
    docker build --tag gcr.io/$PROJECT_ID/reloader:0.1.0 .
    docker push gcr.io/$PROJECT_ID/reloader:0.1.0
    ```

2.  **Deploy to Cloud Run:**

    ```bash
    gcloud run deploy reloader \
      --image=gcr.io/$PROJECT_ID/reloader:0.1.0 \
      --ingress=internal \
      --no-allow-unauthenticated \
      --region=$REGION \
      --service-account=$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com
    ```

### 6. Configure Eventarc Trigger

Create the trigger to invoke the Reloader service when a secret version is added:

```bash
gcloud eventarc triggers create reloader-trigger \
  --destination-run-region=$REGION \
  --destination-run-service=reloader \
  --event-filters="methodName=google.cloud.secretmanager.v1.SecretManagerService.AddSecretVersion" \
  --event-filters="serviceName=secretmanager.googleapis.com" \
  --event-filters="type=google.cloud.audit.log.v1.written" \
  --service-account=$SERVICE_ACCOUNT@$PROJECT_ID.iam.gserviceaccount.com
```

## Local Development

To run the service locally:

```bash
cargo run
```

Note: Local execution will fail to connect to Google Cloud APIs unless you have Application Default Credentials configured (`gcloud auth application-default login`).
