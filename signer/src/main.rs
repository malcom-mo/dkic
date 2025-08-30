use base64::{Engine, engine::general_purpose::STANDARD};
use clap::{Arg, Command};
use ed25519_dalek::{
    Signature, Signer, SigningKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey},
};
use pkcs8::LineEnding;
use rand::rngs::OsRng;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let matches = Command::new("dkic-signer")
        .version("1.0")
        .about("Ed25519 key generation and file signing tool")
        .subcommand(
            Command::new("keygen")
                .about("Generate Ed25519 key pair")
                .arg(
                    Arg::new("out")
                        .long("out")
                        .value_name("PREFIX")
                        .help("Output file prefix for private key PEM file (default: private_key)")
                        .default_value("private_key")
                )
                .arg(
                    Arg::new("outpubkey")
                        .long("outpubkey")
                        .value_name("PREFIXPUB")
                        .help("Output file prefix for public key DNS entry text file (default: public_key)")
                        .default_value("public_key")
                )
        )
        .subcommand(
            Command::new("sign")
                .about("Sign files with Ed25519 private key")
                .arg(
                    Arg::new("private-key")
                        .long("private-key")
                        .value_name("FILE")
                        .help("Path to private key PEM file")
                )
                .arg(
                    Arg::new("files")
                        .help("Files to sign")
                        .required(true)
                        .num_args(1..)
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("keygen", sub_matches)) => {
            let prefix = sub_matches.get_one::<String>("out").unwrap();
            let prefixpub = sub_matches.get_one::<String>("outpubkey").unwrap();
            if let Err(e) = generate_keypair(prefix, prefixpub) {
                eprintln!("Error generating keypair: {}", e);
                std::process::exit(1);
            }
        }
        Some(("sign", sub_matches)) => {
            let private_key_file = sub_matches.get_one::<String>("private-key");
            let files: Vec<_> = sub_matches.get_many::<String>("files").unwrap().collect();

            if let Err(e) = sign_files(private_key_file, &files) {
                eprintln!("Error signing files: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Invalid command. Use --help for usage information.");
            std::process::exit(1);
        }
    }
}

fn generate_keypair(prefix: &str, prefixpub: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let private_key_filename = format!("{}.pem", prefix);
    signing_key.write_pkcs8_pem_file(&private_key_filename, LineEnding::LF)?;

    let public_key_der = signing_key.verifying_key().to_public_key_der()?;
    let public_key_b64 = STANDARD.encode(public_key_der.as_bytes());
    let dns_content = format!("v=DKIC1; k=ed25519; p={}", public_key_b64);
    let public_key_dns = format!("_dkic.[your-domain]. IN TXT \"{}\"", dns_content);
    let public_key_filename = format!("{}.dns.txt", prefixpub);
    fs::write(&public_key_filename, public_key_dns)?;

    println!("Private key: {}", private_key_filename);
    println!("DNS entry with public key: {}:", public_key_filename);
    println!("\tsubdomain: _dkic");
    println!("\ttype: TXT");
    println!("\tcontent: {}", dns_content);

    Ok(())
}

fn sign_files(
    private_key_file: Option<&String>,
    files: &[&String],
) -> Result<(), Box<dyn std::error::Error>> {
    let private_key_pem = if let Some(key_file) = private_key_file {
        fs::read_to_string(key_file)?
    } else {
        env::var("DKIC_PRIVATE_KEY").map_err(
            |_| "No private key file specified and DKIC_PRIVATE_KEY environment variable not set",
        )?
    };
    let private_key = SigningKey::from_pkcs8_pem(&private_key_pem)?;

    for file_path in files {
        if !Path::new(file_path).exists() {
            eprintln!("Warning: File {} does not exist, skipping", file_path);
            continue;
        }

        let original_content = fs::read_to_string(file_path)?;
        let signature: Signature = private_key.sign(original_content.as_bytes());
        let signature_base64 = STANDARD.encode(signature.to_bytes());
        let signature_script = format!(
            r#"<script type="application/json" id="dkic-signature">{{"alg":"ed25519","signature":"{}"}}</script>"#,
            signature_base64
        );

        let modified_content = if let Some(head_end) = original_content.find("</head>") {
            let mut new_content = String::new();
            new_content.push_str(&original_content[..head_end]);
            new_content.push_str(&signature_script);
            new_content.push('\n');
            new_content.push_str(&original_content[head_end..]);
            new_content
        } else {
            // If no </head> found, try to insert after <head>
            if let Some(head_start) = original_content.find("<head>") {
                let insert_pos = head_start + "<head>".len();
                let mut new_content = String::new();
                new_content.push_str(&original_content[..insert_pos]);
                new_content.push('\n');
                new_content.push_str(&signature_script);
                new_content.push_str(&original_content[insert_pos..]);
                new_content
            } else {
                return Err(
                    format!("Could not find <head> section in HTML file: {}", file_path).into(),
                );
            }
        };

        // Write the modified content back to the file
        fs::write(file_path, modified_content)?;

        println!("Signed: {}", file_path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Verifier, VerifyingKey};
    use pkcs8::DecodePublicKey;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_html_signing_and_verification() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path();

        let test_html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Page</title>
    <meta charset="utf-8">
</head>
<body>
    <h1>Hello, World!</h1>
    <p>This is a test HTML file.</p>
</body>
</html>"#;

        let html_file_path = temp_path.join("test.html");
        fs::write(&html_file_path, test_html).expect("Failed to write test HTML file");

        let key_prefix = temp_path.join("test_key");
        let key_prefix_str = key_prefix.to_str().unwrap();
        let key_prefixpub = temp_path.join("test_key_pub");
        let key_prefixpub_str = key_prefixpub.to_str().unwrap();
        generate_keypair(key_prefix_str, key_prefixpub_str).expect("Failed to generate keypair");

        let private_key_path = format!("{}.pem", key_prefix_str);
        let public_key_path = format!("{}.dns.txt", key_prefixpub_str);
        assert!(
            Path::new(&private_key_path).exists(),
            "Private key file not created"
        );
        assert!(
            Path::new(&public_key_path).exists(),
            "Public key file not created"
        );

        let html_file_str = html_file_path.to_str().unwrap().to_string();
        let files = vec![&html_file_str];
        sign_files(Some(&private_key_path), &files).expect("Failed to sign files");

        let signed_html = fs::read_to_string(&html_file_path).expect("Failed to read signed HTML");
        let script_search_str = r#"<script type="application/json" id="dkic-signature">"#;
        assert!(
            signed_html.contains(script_search_str),
            "Signature script not found in signed HTML"
        );

        // Extract the signature from the signed HTML
        let sig_search_str =
            r#"<script type="application/json" id="dkic-signature">{"alg":"ed25519","signature":""#;
        let signature_start = signed_html
            .find(sig_search_str)
            .expect("Signature script start not found");
        let signature_content_start = signature_start + sig_search_str.len();
        let signature_content_end = signed_html[signature_content_start..]
            .find(r#""}"#)
            .expect("Signature script end not found")
            + signature_content_start;

        let signature_base64 = &signed_html[signature_content_start..signature_content_end];
        let signature_bytes = STANDARD
            .decode(signature_base64)
            .expect("Failed to decode signature");
        let signature_array: [u8; 64] = signature_bytes
            .try_into()
            .expect("Signature must be exactly 64 bytes");
        let signature = Signature::from_bytes(&signature_array);

        // Remove the signature script to get original content
        let script_start = signed_html
            .find(script_search_str)
            .expect("Signature script not found for removal");
        let script_end = signed_html[script_start..]
            .find("</script>")
            .expect("Script end tag not found")
            + script_start
            + "</script>".len();

        let mut original_content = String::new();
        original_content.push_str(&signed_html[..script_start]);
        // Remove the newline that was added before the script
        if signed_html.chars().nth(script_start.saturating_sub(1)) == Some('\n') {
            original_content.pop();
        }
        original_content.push_str(&signed_html[script_end..]);

        // Verify the original content matches what we started with
        assert_eq!(
            original_content.trim(),
            test_html.trim(),
            "Original content doesn't match after signature removal"
        );

        // Load the public key and verify the signature
        let public_key_content =
            fs::read_to_string(&public_key_path).expect("Failed to read public key");
        let public_key_b64 = public_key_content
            .trim()
            .strip_prefix("_dkic.[your-domain]. IN TXT \"v=DKIC1; k=ed25519; p=")
            .expect(&format!(
                "Could not parse public_key.dns.txt:\n{}",
                public_key_content
            ))
            .strip_suffix("\"")
            .expect(&format!(
                "Could not parse public_key.dns.txt:\n{}",
                public_key_content
            ));
        let public_key_bytes = STANDARD
            .decode(public_key_b64)
            .expect("Failed to decode public key");
        let public_key =
            VerifyingKey::from_public_key_der(&public_key_bytes).expect("Invalid public key");

        // Verify the signature against the original content
        public_key
            .verify(original_content.as_bytes(), &signature)
            .expect("Signature verification failed");

        println!("✓ Test passed: Keypair generation, signing, and verification successful");
    }
}
