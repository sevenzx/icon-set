import { writable } from 'svelte/store';

export type ToastVariant = 'info' | 'error';

export type ToastMessage = {
  id: number;
  message: string;
  variant: ToastVariant;
};

const toastTimeoutMs = 3000;
let nextToastId = 1;

export const toasts = writable<ToastMessage[]>([]);

export function removeToast(id: number) {
  toasts.update((items) => items.filter((item) => item.id !== id));
}

function pushToast(message: string, variant: ToastVariant) {
  const trimmed = message.trim();
  if (!trimmed) return;

  const id = nextToastId++;
  toasts.update((items) => [...items, { id, message: trimmed, variant }].slice(-4));
  window.setTimeout(() => removeToast(id), toastTimeoutMs);
}

export const toast = {
  info(message: string) {
    pushToast(message, 'info');
  },
  error(message: string) {
    pushToast(message, 'error');
  }
};
