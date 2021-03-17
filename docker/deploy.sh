#!/bin/bash

set -euo pipefail

image_name=$1

sudo apt-get update && sudo apt-get install -y python3-pip python3-setuptools && pip3 install --upgrade --user awscli

# Get login token and execute login
$(aws ecr get-login --no-include-email --region $AWS_REGION)

echo "Tagging latest private image for orderbook...";
docker build --tag $REGISTRY_URI:$image_name  -f docker/Dockerfile.binary .

echo "Pushing private image";
docker push $REGISTRY_URI:$image_name

echo "The private image has been pushed";
rm -rf .ssh/*

echo "Tagging latest private image for db-migrations...";
docker build -t $REGISTRY_URI-migrations -f docker/Dockerfile.migration .

echo "Pushing private image";
docker push $REGISTRY_URI-migrations:$image_name

echo "The private image has been pushed";
rm -rf .ssh/*