function(enabled) {
  // Runs in Aero's isolated world. Preserve each site's own attributes when
  // re-enabling checks; do not turn password fields into spellchecked fields.
  if (!globalThis.__aeroSpellcheckPreference) {
    const original = new WeakMap();
    const changed = new Set();
    let observer;
    let disabled = false;
    const selector = 'html,body,input,textarea,[contenteditable]';
    const visit = (root, fn) => {
      if (root instanceof Element && root.matches(selector)) fn(root);
      for (const element of root.querySelectorAll?.(selector) || []) fn(element);
    };
    const suppress = element => {
      if (!original.has(element)) original.set(element, element.getAttribute('spellcheck'));
      changed.add(element);
      if (element.getAttribute('spellcheck') !== 'false') element.setAttribute('spellcheck', 'false');
    };
    globalThis.__aeroSpellcheckPreference = next => {
      if (next) {
        disabled = false;
        observer?.disconnect();
        for (const element of changed) {
          const attribute = original.get(element);
          if (attribute == null) element.removeAttribute('spellcheck');
          else element.setAttribute('spellcheck', attribute);
          original.delete(element);
        }
        changed.clear();
      } else {
        disabled = true;
        visit(document, suppress);
        observer ??= new MutationObserver(records => {
          if (!disabled) return;
          for (const record of records) {
            if (record.type === 'attributes') visit(record.target, suppress);
            for (const node of record.addedNodes) visit(node, suppress);
            for (const node of record.removedNodes) visit(node, element => changed.delete(element));
          }
        });
        observer.observe(document, {subtree:true,childList:true,attributes:true,attributeFilter:['spellcheck','contenteditable']});
      }
    };
  }
  globalThis.__aeroSpellcheckPreference(enabled);
  return true;
}
