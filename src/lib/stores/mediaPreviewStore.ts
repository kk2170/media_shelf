import { get, writable } from 'svelte/store';

import { loadComicPages, toMediaAssetUrl } from '$lib/tauri';
import type { MediaItem, MediaPreviewData } from '$lib/types';

type PreviewMap = Record<number, MediaPreviewData>;

const pendingPreviewLoads = new Map<number, Promise<void>>();

export const mediaPreviews = writable<PreviewMap>({});

export function ensureMediaPreview(item: MediaItem) {
  const existingPreview = get(mediaPreviews)[item.id];

  if (existingPreview?.status === 'ready' || existingPreview?.status === 'loading') {
    return pendingPreviewLoads.get(item.id) ?? Promise.resolve();
  }

  const assetUrl = toMediaAssetUrl(item.filePath);
  if (!assetUrl) {
    setPreview(item.id, {
      assetUrl: null,
      thumbnailUrl: null,
      pageAssetUrls: null,
      pageCount: item.pageCount ?? null,
      width: null,
      height: null,
      durationSeconds: null,
      status: 'error',
      errorMessage: 'Tauri デスクトップ起動時にプレビューが有効になります。'
    });

    return Promise.resolve();
  }

  setPreview(item.id, {
    assetUrl: item.fileType === 'comic' ? null : assetUrl,
    thumbnailUrl:
      item.fileType === 'image'
        ? assetUrl
        : item.thumbnailUrl
          ? toMediaAssetUrl(item.thumbnailUrl)
          : null,
    pageAssetUrls: null,
    pageCount: item.pageCount ?? null,
    width: null,
    height: null,
    durationSeconds: null,
    status: 'loading',
    errorMessage: null
  });

  const promise = (
    item.fileType === 'image'
      ? loadImagePreview(item.id, assetUrl)
      : item.fileType === 'video'
        ? loadVideoPreview(item.id, assetUrl)
        : loadComicPreview(item)
  )
    .catch((error) => {
      setPreview(item.id, {
        assetUrl: item.fileType === 'comic' ? null : assetUrl,
        thumbnailUrl:
          item.fileType === 'image'
            ? assetUrl
            : item.thumbnailUrl
              ? toMediaAssetUrl(item.thumbnailUrl)
              : null,
        pageAssetUrls: null,
        pageCount: item.pageCount ?? null,
        width: null,
        height: null,
        durationSeconds: null,
        status: 'error',
        errorMessage: error instanceof Error ? error.message : 'プレビュー読み込みに失敗しました。'
      });
    })
    .finally(() => {
      pendingPreviewLoads.delete(item.id);
    });

  pendingPreviewLoads.set(item.id, promise);
  return promise;
}

function setPreview(mediaId: number, preview: MediaPreviewData) {
  mediaPreviews.update((current) => ({
    ...current,
    [mediaId]: preview
  }));
}

function loadImagePreview(mediaId: number, assetUrl: string) {
  return new Promise<void>((resolve, reject) => {
    const image = new Image();

    image.onload = () => {
      setPreview(mediaId, {
        assetUrl,
        thumbnailUrl: assetUrl,
        pageAssetUrls: null,
        pageCount: 1,
        width: image.naturalWidth,
        height: image.naturalHeight,
        durationSeconds: null,
        status: 'ready',
        errorMessage: null
      });
      resolve();
    };

    image.onerror = () => reject(new Error('画像プレビューを読み込めませんでした。'));
    image.src = assetUrl;
  });
}

function loadVideoPreview(mediaId: number, assetUrl: string) {
  return new Promise<void>((resolve, reject) => {
    const video = document.createElement('video');
    video.preload = 'metadata';
    video.muted = true;
    video.playsInline = true;

    let settled = false;
    const timeoutId = window.setTimeout(() => {
      fail('動画メタデータの読み込みがタイムアウトしました。');
    }, 15000);

    const cleanup = () => {
      window.clearTimeout(timeoutId);
      video.pause();
      video.removeAttribute('src');
      video.load();
      video.onloadedmetadata = null;
      video.onseeked = null;
      video.onerror = null;
    };

    const fail = (message: string) => {
      if (settled) return;
      settled = true;
      cleanup();
      reject(new Error(message));
    };

    video.onerror = () => fail('動画プレビューを読み込めませんでした。');

    video.onloadedmetadata = () => {
      const durationSeconds = Number.isFinite(video.duration) ? video.duration : null;
      const width = video.videoWidth || null;
      const height = video.videoHeight || null;

      if (!durationSeconds || durationSeconds <= 0) {
        setPreview(mediaId, {
          assetUrl,
          thumbnailUrl: null,
          pageAssetUrls: null,
          pageCount: 1,
          width,
          height,
          durationSeconds,
          status: 'ready',
          errorMessage: null
        });
        settled = true;
        cleanup();
        resolve();
        return;
      }

      video.currentTime = Math.max(0.1, Math.min(durationSeconds / 10, Math.max(durationSeconds - 0.1, 0.1)));
    };

    video.onseeked = () => {
      if (settled) return;

      try {
        const canvas = document.createElement('canvas');
        canvas.width = video.videoWidth || 320;
        canvas.height = video.videoHeight || 180;

        const context = canvas.getContext('2d');
        if (!context) {
          throw new Error('動画サムネイル描画コンテキストを取得できませんでした。');
        }

        context.drawImage(video, 0, 0, canvas.width, canvas.height);
        const thumbnailUrl = canvas.toDataURL('image/jpeg', 0.82);

        setPreview(mediaId, {
          assetUrl,
          thumbnailUrl,
          pageAssetUrls: null,
          pageCount: 1,
          width: video.videoWidth || null,
          height: video.videoHeight || null,
          durationSeconds: Number.isFinite(video.duration) ? video.duration : null,
          status: 'ready',
          errorMessage: null
        });

        settled = true;
        cleanup();
        resolve();
      } catch (error) {
        fail(error instanceof Error ? error.message : '動画サムネイル生成に失敗しました。');
      }
    };

    video.src = assetUrl;
  });
}

async function loadComicPreview(item: MediaItem) {
  const response = await loadComicPages(item.id);
  const pageAssetUrls = response.pagePaths.map((pagePath) => toMediaAssetUrl(pagePath)).filter(Boolean) as string[];

  if (pageAssetUrls.length === 0) {
    throw new Error('漫画ZIP内に表示可能な画像ページがありませんでした。');
  }

  await new Promise<void>((resolve, reject) => {
    const image = new Image();

    image.onload = () => {
      setPreview(item.id, {
        assetUrl: null,
        thumbnailUrl: pageAssetUrls[0],
        pageAssetUrls,
        pageCount: pageAssetUrls.length,
        width: image.naturalWidth,
        height: image.naturalHeight,
        durationSeconds: null,
        status: 'ready',
        errorMessage: null
      });
      resolve();
    };

    image.onerror = () => reject(new Error('漫画の先頭ページを読み込めませんでした。'));
    image.src = pageAssetUrls[0];
  });
}
