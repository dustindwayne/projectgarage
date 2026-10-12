import type { Snapshot } from './types';
export const native = '__TAURI_INTERNALS__' in window;
export const getServer = () => {
  const value = localStorage.getItem('pg-server') || '';
  // Older builds defaulted to a developer server. Native apps now use their own database.
  if (native && /^http:\/\/(localhost|127\.0\.0\.1|10\.0\.2\.2)(:|\/|$)/.test(value)) return '';
  return value;
};
export const isDeviceMode = () => native && !getServer();
export function setServer(value: string) {
  const v = value.trim().replace(/\/$/, '');
  if (v) {
    const url = new URL(v);
    if (
      url.protocol !== 'https:' &&
      !(url.protocol === 'http:' && ['localhost', '127.0.0.1', '10.0.2.2'].includes(url.hostname))
    )
      throw new Error(
        'Use HTTPS for a shared garage server. Local development addresses may use HTTP.',
      );
  }
  localStorage.setItem('pg-server', v);
}
export const getToken = () => localStorage.getItem('pg-token') || '';
export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}
export async function api<T>(path: string, data?: unknown): Promise<T> {
  if (isDeviceMode()) {
    const { invoke } = await import('@tauri-apps/api/core');
    const result = await invoke<{ status: number; body: T & { error?: string } }>(
      'device_request',
      {
        path,
        token: getToken(),
        data: data ?? null,
      },
    );
    if (result.status >= 400)
      throw new ApiError(result.body.error || 'Could not save on this device', result.status);
    return result.body;
  }
  const response = await fetch(`${getServer()}/api/${path}`, {
    method: data === undefined ? 'GET' : 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${getToken()}` },
    body: data === undefined ? undefined : JSON.stringify(data),
    signal: AbortSignal.timeout(15000),
  });
  const value = await response
    .json()
    .catch(() => ({ error: 'The garage server returned an invalid response' }));
  if (!response.ok)
    throw new ApiError(value.error || 'Could not complete this request', response.status);
  return value as T;
}
function db(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open('project-garage', 1);
    req.onupgradeneeded = () => req.result.createObjectStore('cache');
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}
export async function cache(snapshot: Snapshot) {
  const store = await db();
  await new Promise<void>((resolve, reject) => {
    const tx = store.transaction('cache', 'readwrite');
    tx.objectStore('cache').put(snapshot, `${getServer()}:${snapshot.user.id}`);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  store.close();
  localStorage.setItem('pg-user', snapshot.user.id);
}
export async function cached(): Promise<Snapshot | null> {
  const store = await db();
  return new Promise((resolve, reject) => {
    const req = store
      .transaction('cache')
      .objectStore('cache')
      .get(`${getServer()}:${localStorage.getItem('pg-user')}`);
    req.onsuccess = () => {
      resolve(req.result || null);
      store.close();
    };
    req.onerror = () => reject(req.error);
  });
}
export async function clearSession() {
  const id = localStorage.getItem('pg-user');
  const store = await db();
  await new Promise<void>((resolve) => {
    const tx = store.transaction('cache', 'readwrite');
    tx.objectStore('cache').delete(`${getServer()}:${id}`);
    tx.oncomplete = () => resolve();
    tx.onerror = () => resolve();
  });
  store.close();
  localStorage.removeItem('pg-token');
  localStorage.removeItem('pg-user');
}
export async function imageFile(file: File): Promise<string> {
  if (!['image/jpeg', 'image/png', 'image/webp'].includes(file.type))
    throw new Error('Choose a JPEG, PNG or WebP photo');
  const bitmap = await createImageBitmap(file);
  const ratio = Math.min(1, 1600 / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(bitmap.width * ratio);
  canvas.height = Math.round(bitmap.height * ratio);
  canvas.getContext('2d')!.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  const data = canvas.toDataURL('image/jpeg', 0.82);
  if (data.length > 3_000_000) throw new Error('This photo is too large. Choose a smaller image.');
  return data;
}
export async function receiptFile(file: File): Promise<string> {
  if (file.type !== 'application/pdf') return imageFile(file);
  if (file.size > 2_900_000) throw new Error('Use a PDF receipt smaller than 2.9 MB');
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}
