#!/bin/bash
#
# Script that builds with jekyll and uses dkic-signer on all changed HTML files

# dkic-signer is run in a Docker container that we will try to to build from:
DKIC_SIGNER_PATH=../dkic/signer

echo "Building HTML pages..."
bundle exec jekyll build

# Select all changed .html files under _site (AM = added, modified)
# Set IFS to newline only to handle spaces in filenames
IFS=$'\n'
files=($(git diff --cached --name-only --diff-filter=AM | grep "^_site/.*\.html$" | sed 's|^_site|/_site|'))
unset IFS

if [ ${#files[@]} -eq 0 ]; then
    echo "No modified HTML files to sign"
    exit 0
fi

echo "Building dkic-signer image..."
docker build -t dkic-signer -f $DKIC_SIGNER_PATH

echo "Signing changed HTML files..."
docker run -it -v ./.dkic/private_key.pem:/private_key.pem -v ./_site:/_site dkic-signer sign --private-key /private_key.pem "${files[@]}"
if [ $? -ne 0 ]; then
    echo "dkic-signer failed. Commit aborted."
    exit 1
fi

git add _site

exit 0
