# DKIC - DomainKeys Identified Web Content
CDNs and web hosting providers often ("need to") know the private keys belonging to TLS certificates for their customers' domains.
But as [academic](https://web.archive.org/web/20170108145246id_/http://www.cs.umd.edu:80/~dml/papers/keysharing_ccs16.pdf) [researchers](https://arxiv.org/pdf/2010.16388) and [Hacker News commenters](https://news.ycombinator.com/item?id=44757546) occasionally point out: 

These providers trivially have the power to do man-in-the-middle attacks.

This repo showcases a system to prevent such attacks.
Partially (re-)implementing a [CCS 2022 paper](https://arxiv.org/pdf/2209.01541), it consists of
1. a tool to sign HTML pages using a separate long-term key,
2. a browser extension to verify HTML pages against the domain owner's long-term key.

The domain owner's long-term public key is distributed via DNS.
This makes sense because end users must anyway trust DNS in order to contact the CDN or hosting provider in the first place.
It is also the reason why we call this system DomainKeys Identified Web Content (DKIC) as a reference to [DKIM](https://en.wikipedia.org/wiki/DomainKeys_Identified_Mail).


## Verifier - browser extension
The verifier extension does two things.
First, it looks up a DNS entry that is expected to be of the form
  ```
_dkic.[current-domain]. IN TXT "v=DKIC1; k=ed25519; p=..."
  ```
where `p` is a Base64-encoded Ed25519 public key in DER format.
Second, it parses out a signature from the currently viewed HTML page (of the JSON format shown below) and checks the signature against the public key.

### Usage
To test out the extension (tested in Chrome, Brave and Firefox):

1. Clone this repository
2. Install the extension
    - Chrome/Brave:
        1. Open `chrome://extensions`/`brave://extensions`
        2. Toggle developer mode
        3. Select "Load unpacked" and choose the `verifier` directory in the cloned repo
    - Firefox:
        1. Open `about:debugging#/runtime/this-firefox`
        2. Select "Load Temporary Add-on..." and choose any file in the `verifier` directory in the cloned repo
3. Navigate to any website and open the extension to verify a DKIC signature (eg, https://mal.com.de)

DNS queries are made using a DNS-over-HTTPS (DoH) service that can be configured in the extension's settings.
Currently, only endpoints that support a Google-style JSON API are supported.


## Signer - for static HTML pages
`dkic-signer` is a small command-line tool that can generate keys and sign HTML pages.
It inserts the following into a page's `<head>`:
```html
<script type="application/json" id="dkic-signature">
{
    "alg": "ed25519",
    "signature": "..."
}
</script>
```
where the `"signature"` is a Base64-encoded Ed25519 signature over the rest of the page.

### Usage
```bash
# Install (requires Rust and Cargo)
cargo install --path signer

# Generate a key pair
$ dkic-signer keygen
Private key: private_key.pem
DNS entry with public key: public_key.dns.txt:
        subdomain: _dkic
        type: TXT
        content: v=DKIC1; k=ed25519; p=...

# Sign an HTML page
# Pass the key as an environment variable or use --private-key
$ export DKIC_PRIVATE_KEY=$(cat private_key.pem)
$ dkic-signer sign page1.html page2.html page3.html
Signed: page1.html
Signed: page2.html
Signed: page3.html
```

### Jekyll integration
Here are two options to integrate DKIC signing with Jekyll, the static HTML generator commonly used with Github Pages and Github Action workflows.

#### Option A: local building and signing
The `jekyll` directory contains a script that signs pages built locally with Jekyll.
The idea is to install `build-and-sign.sh` as a git pre-commit hook.
```
cat <dkic repo>/jekyll/build-and-sign.sh >> <your repo>/.git/hook/pre-commit
```
The script currently expects the private key to be in a `.dkic` directory in the repo.
In that case, be sure not to upload the key to Github:
```
echo .dkic >> .gitignore
```

Then, Markdown files are edited, `git add`ed and committed as usual.
Due to the hook script, the pages uploaded to Github will have been signed automatically.

Deploying to Github Pages can be done with the Github Actions workflow in `jekyll/workflow.yml` -- which differs from the standard Actions workflow for Jekyll in only that it does not build the site.

Note that running `jekyll build` as part of the pre-commit script might take an annoyingly long time.

#### Option B: letting the CI runner sign

Alternatively, it would be straightforward to create a CI workflow to sign the generated HTML on the CI server instead of doing it locally.
However, then the key needs to be there, too -- eg, as a [secret](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets) when using Github Actions.
(If the site is hosted *by Github*, ie, if the threat model is that Github would MITM you, then doing this with Github Actions would not make sense.)
