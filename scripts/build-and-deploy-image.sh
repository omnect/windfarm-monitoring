#!/bin/bash
# file used for local development

# In order to deploy the image to the device,
# there must be an existing deployment and the
# restart policy of windfarm-monitoring must be set to `never`!

set -e

# Change directory to the project root (parent of the scripts directory)
cd "$(dirname "$0")/.."

# Default configuration
DEVICE_HOST="${DEVICE_HOST:-}"
DEVICE_USER="${DEVICE_USER:-omnect}"
DEVICE_PASS="${DEVICE_PASS:-}"
IMAGE_TAG="${IMAGE_TAG:-$(whoami)}"
IMAGE_ARCH="${IMAGE_ARCH:-arm64}"
DOCKER_NAMESPACE="${DOCKER_NAMESPACE:-omnectweucopsacr.azurecr.io}"
IMAGE_NAME="omnectshareddevacr.azurecr.io/windfarm-monitoring:${IMAGE_TAG}"
IMAGE_TAR="/tmp/windfarm-monitoring-${IMAGE_ARCH}.tar"

usage() {
  cat << EOF
Usage: $0 [OPTIONS]

Build Docker image for windfarm-monitoring and optionally deploy to device.

OPTIONS:
  --deploy              Deploy the image to the target device after building (requires --host)
  --push                Push the image to the registry after building
  --clean               Perform a clean build without using Docker cache
  --arch <arch>         Target architecture (default: $IMAGE_ARCH)
  --host <hostname>     Target device hostname or IP (required when using --deploy)
  --user <username>     SSH user for target device (default: $DEVICE_USER)
  --password <password> SSH password for target device (default: $DEVICE_PASS)
  --tag <tag>           Docker image tag (default: \$(whoami))
  --namespace <ns>      Docker registry namespace (default: omnectweucopsacr.azurecr.io)
  --help                Show this help message

ENVIRONMENT VARIABLES:
  DEVICE_HOST           Target device hostname or IP (required when using --deploy)
  DEVICE_USER           SSH user for target device (default: omnect)
  DEVICE_PASS           SSH password for target device
  IMAGE_TAG             Docker image tag (default: \$(whoami))
  IMAGE_ARCH            Target architecture (default: arm64)
  DOCKER_NAMESPACE      Docker registry namespace (default: omnectweucopsacr.azurecr.io)

EXAMPLES:
  $0                                    # Build only (arm64)
  $0 --arch amd64                       # Build for amd64
  $0 --clean                            # Clean build without cache
  $0 --push                             # Build and push to registry
  $0 --deploy --host 192.168.1.100      # Build and deploy to specific device
  $0 --deploy --host 192.168.1.100 --password mypassword # Build and deploy with password
  $0 --push --deploy --host 192.168.1.100  # Build, push to registry, and deploy
  $0 --tag v1.2.0 --deploy --host 192.168.1.100  # Build with custom tag and deploy
  DEVICE_HOST=192.168.1.100 $0 --deploy  # Build and deploy using env var
EOF
}

# Parse command line arguments
DEPLOY=false
PUSH=false
CLEAN=false
while [[ $# -gt 0 ]]; do
  case $1 in
    --deploy)
      DEPLOY=true
      shift
      ;;
    --push)
      PUSH=true
      shift
      ;;
    --clean)
      CLEAN=true
      shift
      ;;
    --arch)
      IMAGE_ARCH="$2"
      IMAGE_TAR="/tmp/windfarm-monitoring-${IMAGE_ARCH}.tar"
      shift 2
      ;;
    --host)
      DEVICE_HOST="$2"
      shift 2
      ;;
    --user)
      DEVICE_USER="$2"
      shift 2
      ;;
    --password)
      DEVICE_PASS="$2"
      shift 2
      ;;
    --tag)
      IMAGE_TAG="$2"
      IMAGE_NAME="omnectshareddevacr.azurecr.io/windfarm-monitoring:${IMAGE_TAG}"
      shift 2
      ;;
    --namespace)
      DOCKER_NAMESPACE="$2"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Error: Unknown option $1"
      usage
      exit 1
      ;;
  esac
done

# Validate required parameters
if [[ "$DEPLOY" == "true" && -z "$DEVICE_HOST" ]]; then
  echo "Error: --host is required when using --deploy"
  echo "Provide the device hostname/IP using --host or set DEVICE_HOST environment variable"
  echo ""
  usage
  exit 1
fi

# local build
windfarm_monitoring_version=$(toml get --raw Cargo.toml package.version)

echo "Building ${IMAGE_ARCH} image: $IMAGE_NAME"

# Ensure access to registry
az acr login --name ${DOCKER_NAMESPACE}

# Setup QEMU for cross-architecture builds if needed
if [[ "$IMAGE_ARCH" != "$(uname -m)" ]]; then
  docker run --rm --privileged ${DOCKER_NAMESPACE}/mlilien/qemu-user-static:8.1.2 --reset -p yes
fi

# Build with or without cache
BUILD_ARGS="--platform linux/${IMAGE_ARCH} --load -f Dockerfile . -t $IMAGE_NAME --build-arg DOCKER_NAMESPACE=${DOCKER_NAMESPACE}"
if [[ "$CLEAN" == "true" ]]; then
  echo "Performing clean build (no cache)..."
  docker buildx build --no-cache $BUILD_ARGS
else
  docker buildx build $BUILD_ARGS
fi

# Push to registry if requested
if [[ "$PUSH" == "true" ]]; then
  echo "Pushing image to registry..."
  docker push "$IMAGE_NAME"
  echo "Image pushed successfully: $IMAGE_NAME"
fi

# Deploy to device if requested
if [[ "$DEPLOY" == "true" ]]; then
  echo "Saving Docker image to $IMAGE_TAR..."
  docker save "$IMAGE_NAME" -o "$IMAGE_TAR"

  echo "Copying image to device $DEVICE_HOST..."
  if [ -n "$DEVICE_PASS" ]; then
    sshpass -p "$DEVICE_PASS" scp -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "$IMAGE_TAR" "${DEVICE_USER}@${DEVICE_HOST}:/tmp/"
  else
    scp -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "$IMAGE_TAR" "${DEVICE_USER}@${DEVICE_HOST}:/tmp/"
  fi

  echo "Loading image on device and restarting container..."

  CMD="set -e
    sudo iotedge system stop
    sudo docker container rm -f windfarm-monitoring
    sudo docker image rm -f ${IMAGE_NAME}
    sudo docker load -i /tmp/windfarm-monitoring-${IMAGE_ARCH}.tar
    rm /tmp/windfarm-monitoring-${IMAGE_ARCH}.tar
    sleep 5
    sudo iotedge system restart"

  if [ -n "$DEVICE_PASS" ]; then
    sshpass -p "$DEVICE_PASS" ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "${DEVICE_USER}@${DEVICE_HOST}" "$CMD"
  else
    ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR "${DEVICE_USER}@${DEVICE_HOST}" "$CMD"
  fi

  echo "Cleaning up local tar file..."
  rm -f "$IMAGE_TAR"

  echo "Deployment complete!"
else
  echo "Image built successfully: $IMAGE_NAME"
  echo "To deploy to device, run: $0 --deploy"
  echo "Run '$0 --help' for more options."
fi
