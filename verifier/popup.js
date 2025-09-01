document.addEventListener('DOMContentLoaded', function() {
  const dohUrlSelect = document.getElementById('dohUrl');
  const otherUrlWrapper = document.getElementById('otherUrlWrapper');
  const otherUrlInput = document.getElementById('otherUrl');
  const verifyBtn = document.getElementById('verify-btn');
  const statusDiv = document.getElementById('status');

  // Show/hide custom DoH URL input
  dohUrlSelect.addEventListener("change", () => {
  otherUrlWrapper.style.display = dohUrlSelect.value === "other" ? "block" : "none";
});
  
  function showStatus(message, type = 'info', details = '') {
    statusDiv.className = `status ${type}`;
    statusDiv.innerHTML = message + (details ? `<div class="details">${details}</div>` : '');
  }
  
  function showLoading(message) {
    statusDiv.className = 'status info';
    statusDiv.innerHTML = `<span class="loading"></span>${message}`;
  }
  
  verifyBtn.addEventListener('click', async function() {
    verifyBtn.disabled = true;
    showLoading('Initiating verification...');
    
    try {
      // Get current tab
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      
      if (!tab.url.startsWith('http')) {
        throw new Error('Can only verify HTTP/HTTPS pages');
      }
      
      // Inject content script into the current tab
      await chrome.scripting.executeScript({
        target: { tabId: tab.id },
        files: ['dkic.js']
      });

      let chosenUrl = dohUrlSelect.value;
      if (chosenUrl === 'other') {
        chosenUrl = otherUrlInput.value;
      }
      
      // Send message to content script
      const response = await chrome.tabs.sendMessage(tab.id, { action: 'verifySignature', dohUrl: chosenUrl });
      
      if (response.success) {
        showStatus('✅ Signature verified successfully!', 'success', 
          `Domain: ${response.domain}<br>Public key found in DNS`);
      } else {
        showStatus('❌ Verification failed', 'error', response.error);
      }
      
    } catch (error) {
      showStatus('❌ Error during verification', 'error', error.message);
    } finally {
      verifyBtn.disabled = false;
    }
  });
  
  // Show initial status
  showStatus('Click to verify the current page\'s DKIC signature');
});
