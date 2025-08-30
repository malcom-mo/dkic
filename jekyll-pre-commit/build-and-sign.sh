#!/bin/bash
#
# Script that builds with jekyll and uses dkic-signer on all changed HTML files

echo "Building HTML pages..."
bundle exec jekyll build

# Select all changed .html files under _site (AM = added, modified)
# Set IFS to newline only to handle spaces in filenames
IFS=$'\n'
files=($(git diff --cached --name-only --diff-filter=AM | grep "^_site/.*\.html$"))
unset IFS

if [ ${#files[@]} -eq 0 ]; then
    echo "No modified HTML files to sign"
    exit 0
fi

echo "Building dkic-signer image..."
docker build -t dkic-signer -f $DKIC_SIGNER_PATH

echo "Signing changed HTML files..."
dkic-signer sign --private-key .dkic/private_key.pem "${files[@]}"
if [ $? -ne 0 ]; then
    echo "dkic-signer failed. Commit aborted."
    exit 1
fi

git add _site

exit 0
