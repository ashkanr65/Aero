(() => {
      window.__aeroProtectionConfig = { hideAdElements: true, blockYoutubeVideoAds: true };
      const rules = [
        '[data-ad]', '[data-ads]', '[data-ad-slot]', '[data-ad-client]',
        'ytd-display-ad-renderer', 'ytd-promoted-sparkles-web-renderer',
        'ytd-promoted-video-renderer', 'ytd-ad-slot-renderer',
        'ytd-in-feed-ad-layout-renderer', 'ytd-action-companion-ad-renderer',
        'ytd-companion-slot-renderer', 'ytd-banner-promo-renderer',
        'ytd-statement-banner-renderer', 'ytd-mealbar-promo-renderer',
        'ytd-engagement-panel-section-list-renderer[target-id="engagement-panel-ads"]',
        'ytd-ad-slot-renderer #player-ads', '#masthead-ad',
        'ins.adsbygoogle', 'iframe[title="Advertisement"]',
        'iframe[src*="doubleclick.net"]', 'iframe[src*="googlesyndication.com"]',
        'iframe[src*="googleadservices.com"]',
        'object[type="application/x-shockwave-flash"]',
        'embed[type="application/x-shockwave-flash"]',
        'img[src*="/banners/pr_advertising_ads_banner."]'
      ];
      const applyProtections = () => {
        const config = window.__aeroProtectionConfig;
        if (!config) return;
        for (const element of document.querySelectorAll('[data-aero-hidden-ad]')) {
          if (!config.hideAdElements && element instanceof HTMLElement) {
            element.style.display = element.getAttribute('data-aero-previous-display') || '';
            element.removeAttribute('data-aero-hidden-ad');
            element.removeAttribute('data-aero-previous-display');
          }
        }
        if (config.hideAdElements || config.blockYoutubeVideoAds) {
          const selectors = config.hideAdElements ? rules : rules.slice(10);
          for (const element of document.querySelectorAll(selectors.join(','))) {
            if (element instanceof HTMLElement && !element.hasAttribute('data-aero-hidden-ad')) {
              element.setAttribute('data-aero-previous-display', element.style.display);
              element.setAttribute('data-aero-hidden-ad', '');
              element.style.setProperty('display', 'none', 'important');
            }
          }
        }
        if (config.blockYoutubeVideoAds && /(^|\.)youtube\.com$/.test(location.hostname)) {
          const player = document.querySelector('.html5-video-player');
          if (player?.classList.contains('ad-showing')) {
            const skip = player.querySelector('.ytp-ad-skip-button, .ytp-ad-skip-button-modern, .ytp-skip-ad-button');
            if (skip instanceof HTMLElement && skip.offsetParent !== null) skip.click();
            const video = player.querySelector('video');
            const duration = video instanceof HTMLVideoElement ? video.duration : NaN;
            const remaining = duration - (video instanceof HTMLVideoElement ? video.currentTime : 0);
            if (video instanceof HTMLVideoElement && Number.isFinite(duration) && duration > 0 && remaining > 0 && remaining < 90) {
              video.currentTime = Math.max(0, duration - 0.1);
            }
          }
        }
      };
      if (!window.__aeroProtectionObserver && document.documentElement) {
        window.__aeroProtectionObserver = new MutationObserver(applyProtections);
        window.__aeroProtectionObserver.observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ['class'] });
        window.__aeroProtectionInterval = window.setInterval(applyProtections, 750);
      }
      applyProtections();
    })();;window.__aeroReadAloud=()=>{if(speechSynthesis.speaking){speechSynthesis.cancel();return;}speechSynthesis.speak(new SpeechSynthesisUtterance((document.querySelector('article')||document.body).innerText.slice(0,30000)))};window.__aeroReader=()=>{const article=document.querySelector('article,main');if(!article)return;document.body.innerHTML=article.outerHTML;document.body.style.cssText='max-width:760px;margin:48px auto;padding:24px;font:19px/1.75 Georgia;background:#faf8f2;color:#202124';};