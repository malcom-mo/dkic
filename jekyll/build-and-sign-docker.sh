#!/bin/bash
#
# Script that builds with jekyll and signs in a Docker container

# Confirmation prompt:
# read -p "Sign HTML pages with DKIC? [y/N]: " answer </dev/tty
# answer=$(echo "$answer" | tr '[:upper:]' '[:lower:]')
# if [[ "$answer" != "y" && "$answer" != "yes" ]]; then
#     exit 0
# fi

# dkic-signer is run in a Docker container that we will try to to build from:
DKIC_SIGNER_PATH=../dkic/signer

echo "Building HTML pages..."
# TODO: does --incremental actually work?
bundle exec jekyll build --quiet --incremental
git add -f _site

# Select all changed .html files under _site (AM = added, modified)
# Set IFS to newline only to handle spaces in filenames
IFS=$'\n'
files=($(git diff --staged --name-only --diff-filter=AM | grep "^_site/.*\.html$" | sed 's|^_site|/_site|'))
unset IFS

if [ ${#files[@]} -eq 0 ]; then
    echo "No modified HTML files to sign"
    exit 0
fi

echo "Building dkic-signer image..."
docker build --tag dkic-signer --quiet $DKIC_SIGNER_PATH

echo "Signing changed HTML files..."
docker run --rm -v ./.dkic/private_key.pem:/private_key.pem -v ./_site:/_site dkic-signer sign --private-key /private_key.pem "${files[@]}"
if [ $? -ne 0 ]; then
    echo "dkic-signer failed. Commit aborted."
    exit 1
fi

git add -f _site

# Confirmation prompt:
# read -p "Commit now? [y/N]: " answer </dev/tty
# answer=$(echo "$answer" | tr '[:upper:]' '[:lower:]')
# if [[ "$answer" != "y" && "$answer" != "yes" ]]; then
#     exit 1
# fi

exit 0
