// Proactive token refresh for SaaS mode.
// Refreshes the access_token cookie every 13 minutes via the parent API.
(function () {
  'use strict';

  var REFRESH_INTERVAL_MS = 13 * 60 * 1000; // 13 minutes
  var RETRY_INTERVAL_MS = 30 * 1000; // 30 seconds on failure
  var timerId = null;

  // The refresh URL is injected by the server at serve time.
  var refreshUrl = '{{SAAS_REFRESH_URL}}';

  if (!refreshUrl || refreshUrl.indexOf('{{') === 0) return;

  function doRefresh() {
    fetch(refreshUrl, { method: 'POST', credentials: 'include' })
      .then(function (r) {
        schedule(r.ok ? REFRESH_INTERVAL_MS : RETRY_INTERVAL_MS);
      })
      .catch(function () {
        schedule(RETRY_INTERVAL_MS);
      });
  }

  function schedule(ms) {
    if (timerId) clearTimeout(timerId);
    timerId = setTimeout(doRefresh, ms);
  }

  // Refresh immediately when a backgrounded tab becomes visible
  document.addEventListener('visibilitychange', function () {
    if (document.visibilityState === 'visible') {
      doRefresh();
    }
  });

  // Start the proactive refresh timer
  schedule(REFRESH_INTERVAL_MS);
})();
