"DKIM for HTML pages" PoC

# DKIC - DomainKeys Identified Web Content
CDNs and web hosting providers often ("need to") know the private keys belonging to TLS certificates for their customers' domains.
But as the [research](earlyStudy) [literature](tlsInterception) and the occasional [Hacker News thread](https://news.ycombinator.com/item?id=44755528) point out: 

These providers trivially have the power to do man-in-the-middle attacks.

This repo showcases a system to prevent such attacks.
It is partial re-implementation of a [CCS 2022 paper](invicloak) and consists of
1. a tool to sign HTML pages using a separate long-term key,
2. a browser plugin to verify HTML pages against the domain owner's long-term key.

The domain owner's long-term public key is distributed via DNS.
This makes sense because end users must anyway trust DNS in order to contact the CDN or hosting provider in the first place.
It is also the reason why we call this system DomainKeys Identified Web Content (DKIC) as a reference to [DKIM](dkim).


## Signer - for static HTML
`dkic-signer` is a small command-line tool that can generate keys and sign HTML pages.
It inserts the following into a page's `<head>`:
```json
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
# Compile (requires Rust and Cargo)
cargo build --release  # output in target/release/dkic-signer

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
