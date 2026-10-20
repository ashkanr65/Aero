let gesture = null;
const visible = (field) => {
    if (field.disabled || field.readOnly || field.closest('[hidden], [inert], [aria-hidden="true"]'))
        return false;
    const style = getComputedStyle(field), rect = field.getBoundingClientRect();
    return style.display !== 'none' && style.visibility === 'visible' && Number(style.opacity) !== 0 && rect.width > 0 && rect.height > 0;
};
document.addEventListener('click', event => {
    if (!event.isTrusted || !(event.target instanceof Element))
        return;
    const control = event.target.closest('button, input');
    if ((control instanceof HTMLButtonElement || control instanceof HTMLInputElement) && /^(submit|image)$/.test(control.type) && control.form)
        gesture = { form: control.form, time: Date.now() };
}, true);
document.addEventListener('keydown', event => {
    if (event.isTrusted && event.key === 'Enter' && event.target instanceof HTMLInputElement && event.target.form)
        gesture = { form: event.target.form, time: Date.now() };
}, true);
document.addEventListener('submit', event => {
    const approval = gesture;
    gesture = null;
    if (!event.isTrusted || window.top !== window || !(event.target instanceof HTMLFormElement) || !approval || approval.form !== event.target || Date.now() - approval.time > 1500)
        return;
    const form = event.target;
    const fields = [...form.querySelectorAll('input')].filter(field => field.form === form && visible(field));
    const password = fields.find(field => field.type === 'password');
    const username = fields.find(field => /^(email|text|tel)$/.test(field.type) && /user|login|email|account|identifier/i.test([field.name, field.id, field.autocomplete].join(' ')));
    let action;
    try {
        action = new URL(form.action || location.href, location.href);
    }
    catch {
        return;
    }
    if (location.protocol !== 'https:' || action.protocol !== 'https:' || action.origin !== location.origin || (form.target && form.target !== '_self') || !password?.value || !username?.value)
        return;
    window.__AeroLoginSubmission(JSON.stringify({ origin: location.origin, username: username.value, password: password.value }));
}, true);
