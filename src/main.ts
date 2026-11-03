import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

// The system webview's browser navigation/printing menu is not meaningful in
// a desktop file manager. Keep the native editing menu on text fields, where
// users still need operations such as copy and paste.
document.addEventListener('contextmenu', (event) => {
  const target = event.target;
  if (!(target instanceof Element)) {
    event.preventDefault();
    return;
  }
  const editable = target.closest('input, textarea, [contenteditable]');
  if (editable instanceof HTMLInputElement) {
    if (
      !editable.disabled &&
      ['text', 'search', 'password', 'email', 'url', 'tel', 'number'].includes(editable.type)
    )
      return;
  } else if (editable instanceof HTMLTextAreaElement) {
    if (!editable.disabled) return;
  } else if (editable instanceof HTMLElement && editable.isContentEditable) {
    return;
  }
  event.preventDefault();
});

mount(App, { target: document.getElementById('app')! });
