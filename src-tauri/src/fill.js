function(params){ return (() => {
    const approvedUrl = params.url;
    const approvedOrigin = params.origin;
    if (location.href !== approvedUrl || location.origin !== approvedOrigin || location.protocol !== 'https:' || window.top !== window) return false;
    const visible = element => {
      if (!(element instanceof HTMLInputElement) || !element.isConnected || element.disabled || element.readOnly || element.type === 'hidden' || element.closest('[hidden], [inert], [aria-hidden="true"]')) return false;
      for (let ancestor = element; ancestor instanceof Element; ancestor = ancestor.parentElement) {
        const style = getComputedStyle(ancestor);
        if (style.display === 'none' || style.visibility !== 'visible' || Number(style.opacity) === 0) return false;
      }
      const rect = element.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0 || rect.right <= 0 || rect.bottom <= 0 || rect.left >= innerWidth || rect.top >= innerHeight) return false;
      const x = (Math.max(0, rect.left) + Math.min(innerWidth, rect.right)) / 2;
      const y = (Math.max(0, rect.top) + Math.min(innerHeight, rect.bottom)) / 2;
      return document.elementFromPoint(x, y) === element;
    };
    const passwordField = [...document.querySelectorAll('input[type="password"]')].find(visible);
    if (!passwordField) return false;
    const form = passwordField.form;
    const action = new URL(form?.action || location.href, location.href);
    if (action.protocol !== 'https:' || action.origin !== approvedOrigin || (form?.target && form.target !== '_self')) return false;
    const fields = [...(form || document).querySelectorAll('input')].filter(visible);
    const userField = fields.find(field => field.form === form && field !== passwordField && /^(email|text|tel)$/i.test(field.type) && /user|login|email|account|identifier/i.test([field.name, field.id, field.autocomplete, field.placeholder].join(' ')));
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    if (!setter || location.href !== approvedUrl || !visible(passwordField)) return false;
    // Set both values before events: a username handler must not redirect a subsequent secret write.
    if (userField) setter.call(userField, params.username);
    setter.call(passwordField, params.password);
    for (const field of [userField, passwordField].filter(Boolean)) {
      field.dispatchEvent(new Event('input', { bubbles: true }));
      field.dispatchEvent(new Event('change', { bubbles: true }));
    }
    return true;
  })(); }