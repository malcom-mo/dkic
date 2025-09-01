#!/bin/bash
#
# Script that builds with jekyll and uses dkic-signer on all changed HTML files

# Confirmation prompt:
# read -p "Sign HTML pages with DKIC? [y/N]: " answer </dev/tty
# answer=$(echo "$answer" | tr '[:upper:]' '[:lower:]')
# if [[ "$answer" != "y" && "$answer" != "yes" ]]; then
#     exit 0
# fi

echo "Building HTML pages..."
# TODO: does --incremental actually work?
bundle exec jekyll build --quiet --incremental
git add -f _site

# Select all changed .html files under _site (AM = added, modified)
# Set IFS to newline only to handle spaces in filenames
IFS=$'\n'
files=($(git diff --staged --name-only --diff-filter=AM | grep "^_site/.*\.html$"))
unset IFS

if [ ${#files[@]} -eq 0 ]; then
    echo "No modified HTML files to sign"
    exit 0
fi

echo "Signing changed HTML files..."
dkic-signer sign --private-key .dkic/private_key.pem "${files[@]}"
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
