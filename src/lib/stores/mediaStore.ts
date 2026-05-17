import { derived, get, writable } from 'svelte/store';
import type { MediaItem } from '$lib/types';

const initialMedia: MediaItem[] = [];

export const mediaItems = writable<MediaItem[]>(initialMedia);
export const selectedMediaId = writable<number | null>(null);

export const selectedMedia = derived(
  [mediaItems, selectedMediaId],
  ([$mediaItems, $selectedMediaId]) =>
    $mediaItems.find((item) => item.id === $selectedMediaId) ?? null
);

export function setMediaItems(items: MediaItem[]) {
  mediaItems.set(items);

  const currentSelectedId = get(selectedMediaId);
  const hasCurrentSelection = items.some((item) => item.id === currentSelectedId);

  if (items.length === 0) {
    selectedMediaId.set(null);
    return;
  }

  if (!hasCurrentSelection) {
    selectedMediaId.set(items[0].id);
  }
}

export function setSelectedMediaId(mediaId: number | null) {
  selectedMediaId.set(mediaId);
}
