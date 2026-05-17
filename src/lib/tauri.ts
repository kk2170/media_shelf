import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

import type {
  ComicPagesResponse,
  ImportMediaResult,
  LibrarySnapshot,
  MediaItem,
  SourceMetadataResponse,
  Tag,
  TagCategory
} from '$lib/types';

export function isTauriApp() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export async function pickImportDirectory() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '取り込み元フォルダを選択'
  });

  return typeof selected === 'string' ? selected : null;
}

export async function loadMediaItems() {
  return invoke<MediaItem[]>('load_media_items');
}

export async function loadLibrarySnapshot() {
  return invoke<LibrarySnapshot>('load_library_snapshot');
}

export async function importMediaFolder(folderPath: string) {
  return invoke<ImportMediaResult>('import_media_folder', { folderPath });
}

export async function createTagCategory(name: string, color: string | null) {
  return invoke<TagCategory>('create_tag_category', { name, color });
}

export async function createTag(
  categoryId: number,
  name: string,
  description: string | null,
  color: string | null
) {
  return invoke<Tag>('create_tag', { categoryId, name, description, color });
}

export async function setMediaTags(mediaId: number, tagIds: number[]) {
  return invoke<MediaItem>('set_media_tags', { mediaId, tagIds });
}

export async function setComicProgress(mediaId: number, pageIndex: number) {
  return invoke<MediaItem>('set_comic_progress', { mediaId, pageIndex });
}

export async function loadComicPages(mediaId: number) {
  return invoke<ComicPagesResponse>('load_comic_pages', { mediaId });
}

export async function setMediaSeries(mediaId: number, seriesName: string) {
  return invoke<MediaItem>('set_media_series', { mediaId, seriesName });
}

export async function setMediaSourceFromUrl(mediaId: number, sourceUrl: string) {
  return invoke<SourceMetadataResponse>('set_media_source_from_url', { mediaId, sourceUrl });
}

export async function touchLastViewed(mediaId: number) {
  return invoke<MediaItem>('touch_last_viewed', { mediaId });
}

export function toMediaAssetUrl(filePath: string) {
  if (!isTauriApp()) {
    return null;
  }

  return convertFileSrc(filePath);
}
