// DKIC signature verification

const DKIC_SUBDOMAIN = '_dkic';

// Ed25519 verification using Web Crypto API
class Ed25519Verifier {
  static async importPublicKey(derBytes) {
    try {
      // Ed25519 public keys in DER format have a specific ASN.1 structure
      // For simplicity, we'll assume the last 32 bytes are the raw public key
      const rawKey = derBytes.slice(-32);
      
      return await crypto.subtle.importKey(
        'raw',
        rawKey,
        {
          name: 'Ed25519',
        },
        false,
        ['verify']
      );
    } catch (error) {
      throw new Error(`Failed to import Ed25519 public key: ${error.message}`);
    }
  }
  
  static async verify(publicKey, signature, data) {
    try {
      return await crypto.subtle.verify(
        'Ed25519',
        publicKey,
        signature,
        new TextEncoder().encode(data)
      );
    } catch (error) {
      throw new Error(`Signature verification failed: ${error.message}`);
    }
  }
}

// DNS TXT record lookup using DNS over HTTPS
async function lookupTxtRecord(dohUrl, domain) {
  const dnsQuery = `${DKIC_SUBDOMAIN}.${domain}`;
  const url = `${dohUrl}?name=${encodeURIComponent(dnsQuery)}&type=TXT`;
  
  try {
    const response = await fetch(url, {
      headers: {
        'Accept': 'application/dns-json'
      }
    });
    
    if (!response.ok) {
      throw new Error(`DNS lookup failed: ${response.status} ${response.statusText}`);
    }
    
    const data = await response.json();
    
    if (data.Status !== 0) {
      throw new Error(`DNS query failed with status: ${data.Status}`);
    }
    
    if (!data.Answer || data.Answer.length === 0) {
      throw new Error(`No TXT record found for ${dnsQuery}`);
    }
    
    // Get the first TXT record and remove quotes
    let txtData = data.Answer[0].data;
    if (txtData.startsWith('"') && txtData.endsWith('"')) {
      txtData = txtData.slice(1, -1);
    }
    
    return txtData;
    
  } catch (error) {
    throw new Error(`DNS lookup error: ${error.message}`);
  }
}

// Parse DNS TXT record format: v=1; k=ed25519; p=<base64-key>
function parseDnsRecord(txtRecord) {
  try {
    // Split by semicolon and parse key-value pairs
    const pairs = txtRecord.split(';').map(pair => pair.trim());
    const parsed = {};
    
    for (const pair of pairs) {
      const [key, value] = pair.split('=').map(s => s.trim());
      if (key && value) {
        parsed[key] = value;
      }
    }
    
    // Validate required fields
    if (!parsed.v || parsed.v !== 'DKIC1') {
      throw new Error(`Unsupported version: ${parsed.v}. Expected v=DKIC1`);
    }
    
    if (!parsed.k || parsed.k !== 'ed25519') {
      throw new Error(`Unsupported key type: ${parsed.k}. Expected k=ed25519`);
    }
    
    if (!parsed.p) {
      throw new Error('Missing public key field (p=) in DNS record');
    }
    
    return parsed.p;
    
  } catch (error) {
    throw new Error(`Invalid DNS record format: ${error.message}`);
  }
}

// Base64 decoding
function base64ToBytes(base64) {
  try {
    const binaryString = atob(base64);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  } catch (error) {
    throw new Error(`Invalid base64 encoding: ${error.message}`);
  }
}

// Extract signature data from page
function extractSignatureData() {
  const scriptElement = document.getElementById('dkic-signature');
  if (!scriptElement) {
    throw new Error('No signature data found (missing script#dkic-signature element)');
  }
  
  if (scriptElement.type !== 'application/json') {
    throw new Error('Signature script element must have type="application/json"');
  }
  
  try {
    const signatureData = JSON.parse(scriptElement.textContent);
    if (!signatureData.signature) {
      throw new Error('Signature data missing "signature" field');
    }
    return signatureData;
  } catch (error) {
    throw new Error(`Invalid signature JSON: ${error.message}`);
  }
}

// Get original HTML source (not live DOM)
async function getOriginalHtmlSource() {
  try {
    // Fetch the current page's HTML source
    const response = await fetch(window.location.href);
    if (!response.ok) {
      throw new Error(`Failed to fetch page source: ${response.status}`);
    }
    const originalHtml = await response.text();
    return originalHtml;
  } catch (error) {
    throw new Error(`Cannot fetch original HTML: ${error.message}`);
  }
}

// Remove signature script from HTML string using string manipulation
function removeSignatureFromHtml(htmlString) {
  // Find and remove the signature script element (and trailing whitespace)
  const signatureScriptRegex = /<script[^>]*\sid=["']dkic-signature["'][^>]*>[\s\S]*?<\/script>\s*/gi;  
  // Remove the signature script element
  const cleanedHtml = htmlString.replace(signatureScriptRegex, '');
  
  return cleanedHtml;
}

// Get HTML content without signature script (using original source)
async function getVerifiableHtml() {
  // Get original HTML source
  const originalHtml = await getOriginalHtmlSource();
  
  // Remove signature script
  const verifiableHtml = removeSignatureFromHtml(originalHtml);
  
  return verifiableHtml;
}

// Extract domain from current URL
function getCurrentDomain() {
  try {
    const url = new URL(window.location.href);
    return url.hostname;
  } catch (error) {
    throw new Error(`Invalid URL: ${error.message}`);
  }
}

// Main verification function
async function verifyHtmlSignature(dohUrl) {
  try {
    console.log('Starting DKIC signature verification...');
    
    const domain = getCurrentDomain();
    console.log(`Current domain: ${domain}`);
    
    const signatureData = extractSignatureData();
    console.log('Signature data extracted');
    
    const htmlContent = await getVerifiableHtml();
    console.log(`HTML content length: ${htmlContent.length} characters`);

    console.log(`Looking up TXT record for: ${DKIC_SUBDOMAIN}.${domain}`);
    const txtRecord = await lookupTxtRecord(dohUrl, domain);
    console.log('DNS TXT record retrieved');
    
    const base64PublicKey = parseDnsRecord(txtRecord);
    const publicKeyDer = base64ToBytes(base64PublicKey);
    console.log(`Public key DER length: ${publicKeyDer.length} bytes`);
    
    const publicKey = await Ed25519Verifier.importPublicKey(publicKeyDer);
    console.log('Public key imported successfully');
    
    const signatureBytes = base64ToBytes(signatureData.signature);
    console.log(`Signature length: ${signatureBytes.length} bytes`);
    
    const isValid = await Ed25519Verifier.verify(publicKey, signatureBytes, htmlContent);
    
    if (isValid) {
      console.log('✅ Signature verification successful!');
      return { success: true, domain, htmlContent };
    } else {
      throw new Error('Signature verification failed - signature does not match content');
    }
    
  } catch (error) {
    console.error('❌ Signature verification error:', error);
    return { success: false, error: error.message };
  }
}

// Listen for messages from popup
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  if (request.action === 'verifySignature') {
    verifyHtmlSignature(request.dohUrl)
      .then(result => sendResponse(result))
      .catch(error => sendResponse({ success: false, error: error.message }));
    
    // Return true to indicate we'll respond asynchronously
    return true;
  }
});

console.log('DKIC Verifier script loaded');
