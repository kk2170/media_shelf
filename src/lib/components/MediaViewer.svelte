<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import { ensureMediaPreview, mediaPreviews } from '$lib/stores/mediaPreviewStore';

  import type { MediaItem } from '$lib/types';

  const dispatch = createEventDispatcher<{
    comicProgress: { mediaId: number; pageIndex: number };
    resolveSource: { sourceUrl: string };
    updateSeries: { mediaId: number; seriesName: string };
  }>();

  export let item: MediaItem | null = null;
  export let busy = false;
  export let progressMessage = '';
  let currentComicPageIndex = 0;
  let previousItemId: number | null = null;
  let sourceUrlInput = '';
  let seriesNameInput = '';

  $: if (item) {
    ensureMediaPreview(item);
  }

  $: if (!isComic(item)) {
    currentComicPageIndex = 0;
  }

  $: if (item?.id !== previousItemId) {
    previousItemId = item?.id ?? null;
    currentComicPageIndex = item?.currentPageIndex ?? 0;
    sourceUrlInput = item?.sourceUrl ?? '';
    seriesNameInput = item?.seriesName ?? '';
  }

  $: {
    const preview = getPreview(item);
    if (preview?.pageAssetUrls && isComic(item)) {
      preloadNearbyComicPages(preview.pageAssetUrls, currentComicPageIndex);
    }
  }

  function formatFileSize(size?: number | null) {
    if (size == null) return '不明';

    const units = ['B', 'KB', 'MB', 'GB'];
    let value = size;
    let unitIndex = 0;

    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }

    return `${value.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`;
  }

  function formatImportedAt(value: string) {
    const epochSeconds = Number(value);

    if (Number.isFinite(epochSeconds) && epochSeconds > 0) {
      return new Date(epochSeconds * 1000).toLocaleString('ja-JP');
    }

    return value;
  }

  function getPreview(currentItem: MediaItem | null) {
    return currentItem ? $mediaPreviews[currentItem.id] : null;
  }

  function isComic(currentItem: MediaItem | null) {
    return currentItem?.fileType === 'comic';
  }

  function displayName(currentItem: MediaItem | null) {
    if (!currentItem) {
      return '';
    }

    return isComic(currentItem) && currentItem.sourceTitle ? currentItem.sourceTitle : currentItem.fileName;
  }

  function comicReadStatus(currentItem: MediaItem | null): 'unread' | 'reading' | 'completed' | null {
    if (!isComic(currentItem)) {
      return null;
    }

    const pageCount = currentItem?.pageCount ?? 0;
    const currentPageIndex = currentItem?.currentPageIndex ?? 0;

    if (pageCount <= 0 || currentPageIndex <= 0) {
      return 'unread';
    }

    if (currentPageIndex >= pageCount - 1) {
      return 'completed';
    }

    return 'reading';
  }

  function comicReadStatusLabel(currentItem: MediaItem | null) {
    const status = comicReadStatus(currentItem);
    if (status === 'completed') return '読了';
    if (status === 'reading') return '読書中';
    if (status === 'unread') return '未読';
    return '';
  }

  function formatDuration(value: number | null | undefined) {
    if (value == null || Number.isNaN(value)) {
      return '不明';
    }

    const totalSeconds = Math.max(0, Math.round(value));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    return hours > 0
      ? `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
      : `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }

  function moveComicPage(offset: number, pageCount: number) {
    updateComicProgress(Math.min(Math.max(currentComicPageIndex + offset, 0), pageCount - 1));
  }

  function goToComicPage(pageIndex: number, pageCount: number) {
    updateComicProgress(Math.min(Math.max(pageIndex, 0), pageCount - 1));
  }

  function updateComicProgress(nextPageIndex: number) {
    if (nextPageIndex === currentComicPageIndex) {
      return;
    }

    currentComicPageIndex = nextPageIndex;
    if (item) {
      dispatch('comicProgress', { mediaId: item.id, pageIndex: nextPageIndex });
    }
  }

  function preloadNearbyComicPages(pageAssetUrls: string[], currentIndex: number) {
    const nearbyIndexes = [currentIndex - 1, currentIndex + 1].filter(
      (index) => index >= 0 && index < pageAssetUrls.length
    );

    for (const index of nearbyIndexes) {
      const image = new Image();
      image.src = pageAssetUrls[index];
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (!isComic(item)) {
      return;
    }

    const preview = getPreview(item);
    const pageCount = preview?.pageAssetUrls?.length ?? 0;
    if (pageCount === 0) {
      return;
    }

    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      moveComicPage(-1, pageCount);
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      moveComicPage(1, pageCount);
    } else if (event.key === 'Home') {
      event.preventDefault();
      goToComicPage(0, pageCount);
    } else if (event.key === 'End') {
      event.preventDefault();
      goToComicPage(pageCount - 1, pageCount);
    }
  }

  function submitSourceUrl() {
    const normalized = sourceUrlInput.trim();
    if (!normalized) {
      return;
    }

    dispatch('resolveSource', { sourceUrl: normalized });
  }

  function submitSeriesName() {
    const normalized = seriesNameInput.trim();
    if (!normalized || !item) {
      return;
    }

    dispatch('updateSeries', { mediaId: item.id, seriesName: normalized });
  }
</script>

<svelte:window on:keydown={handleWindowKeydown} />

<section class="panel viewer">
  <div>
    <h2>メディア表示</h2>
    <p>画像は拡大表示、動画は再生、漫画ZIPはページ送りで閲覧できます。</p>
  </div>

  {#if item}
    {@const preview = getPreview(item)}
    <div class="viewer-card">
      <div class="stage">
        {#if preview?.pageAssetUrls && isComic(item)}
          {@const comicPageCount = preview.pageAssetUrls.length}
          <div class="comic-stage">
            <img
              alt={`${displayName(item)} ${currentComicPageIndex + 1}ページ目`}
              src={preview.pageAssetUrls[currentComicPageIndex]}
            />

            <div class="comic-controls">
              <button disabled={currentComicPageIndex === 0} on:click={() => goToComicPage(0, comicPageCount)}>
                最初へ
              </button>
              <button disabled={currentComicPageIndex === 0} on:click={() => moveComicPage(-1, comicPageCount)}>
                前へ
              </button>
              <span>{currentComicPageIndex + 1} / {comicPageCount} ページ</span>
              <button
                disabled={currentComicPageIndex >= comicPageCount - 1}
                on:click={() => moveComicPage(1, comicPageCount)}
              >
                次へ
              </button>
              <button
                disabled={currentComicPageIndex >= comicPageCount - 1}
                on:click={() => goToComicPage(comicPageCount - 1, comicPageCount)}
              >
                最後へ
              </button>
            </div>
            <small class="comic-hint">← / → キーでページ送り、Home / End で先頭・末尾へ移動できます。</small>
          </div>
        {:else if preview?.assetUrl && item.fileType === 'image'}
          <img alt={displayName(item)} src={preview.assetUrl} />
        {:else if preview?.assetUrl && item.fileType === 'video'}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            controls
            playsinline
            poster={preview.thumbnailUrl ?? undefined}
            preload="metadata"
            src={preview.assetUrl}
          ></video>
        {:else if preview?.status === 'loading'}
          <div class="placeholder">プレビューとメタデータを読み込み中です...</div>
        {:else}
          <div class="placeholder">{preview?.errorMessage ?? 'プレビューを読み込めませんでした。'}</div>
        {/if}
      </div>

      <strong>{displayName(item)}</strong>
      {#if isComic(item) && item.sourceTitle}
        <small>元ファイル名: {item.fileName}</small>
      {/if}
      <span>{item.fileType === 'video' ? '動画' : item.fileType === 'comic' ? '漫画ZIP' : '画像'}</span>
      {#if isComic(item)}
        <span>状態: {comicReadStatusLabel(item)}</span>
        <span>ページ数: {preview?.pageCount ?? item.pageCount ?? '不明'}</span>
      {:else if preview?.width != null && preview?.height != null}
        <span>解像度: {preview.width} × {preview.height}</span>
      {/if}
      {#if item.fileType === 'video'}
        <span>長さ: {formatDuration(preview?.durationSeconds)}</span>
      {/if}
      <span>管理方式: {item.sourcePath ? 'コピー管理' : '外部参照（旧データ）'}</span>
      <span>サイズ: {formatFileSize(item.fileSize)}</span>
      <span>取り込み時刻: {formatImportedAt(item.importedAt)}</span>
      {#if item.completedAt}
        <span>読了時刻: {formatImportedAt(item.completedAt)}</span>
      {/if}
      {#if item.lastViewedAt}
        <span>最終閲覧: {formatImportedAt(item.lastViewedAt)}</span>
      {/if}
      {#if isComic(item)}
        <div class="source-block">
          <strong>シリーズ名</strong>
          <div class="source-form">
            <input bind:value={seriesNameInput} placeholder="シリーズ名を入力" />
            <button disabled={busy || seriesNameInput.trim().length === 0} on:click={submitSeriesName}>
              保存
            </button>
          </div>
          {#if item.seriesName}
            <span>シリーズ: {item.seriesName}</span>
          {/if}
        </div>
        <div class="source-block">
          <strong>漫画の元URL / タイトル</strong>
          <div class="source-form">
            <input bind:value={sourceUrlInput} placeholder="漫画ページのURLを貼り付け" />
            <button disabled={busy || sourceUrlInput.trim().length === 0} on:click={submitSourceUrl}>
              {busy ? '取得中...' : 'タイトル取得'}
            </button>
          </div>
          {#if item.sourceTitle}
            <span>タイトル: {item.sourceTitle}</span>
          {/if}
          {#if item.sourceUrl}
            <small>URL: {item.sourceUrl}</small>
          {/if}
          {#if progressMessage}
            <small class="progress-message">{progressMessage}</small>
          {/if}
        </div>
      {/if}
      {#if item.sourcePath}
        <small>元ファイル: {item.sourcePath}</small>
      {/if}
      <small>管理ファイル: {item.filePath}</small>
    </div>
  {:else}
    <div class="placeholder">メディアを選択すると詳細が表示されます。</div>
  {/if}
</section>

<style>
  .panel {
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    background: rgba(15, 23, 42, 0.65);
  }

  h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  p {
    margin: 0.35rem 0 0;
    color: #94a3b8;
  }

  .placeholder,
  .viewer-card {
    display: grid;
    gap: 0.4rem;
    padding: 1.2rem;
    border-radius: 16px;
    background: rgba(30, 41, 59, 0.72);
  }

  .placeholder {
    min-height: 180px;
    align-content: center;
  }

  .stage {
    display: grid;
    place-items: center;
    overflow: hidden;
    min-height: 320px;
    border-radius: 14px;
    background: rgba(15, 23, 42, 0.65);
    margin-bottom: 0.6rem;
  }

  .stage img,
  .stage video {
    max-width: 100%;
    max-height: 460px;
    width: 100%;
    object-fit: contain;
  }

  .comic-stage {
    display: grid;
    gap: 0.9rem;
    width: 100%;
  }

  .comic-controls {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .comic-controls button {
    border: none;
    border-radius: 999px;
    padding: 0.55rem 0.9rem;
    background: #38bdf8;
    color: #0f172a;
    font: inherit;
    cursor: pointer;
  }

  .comic-controls button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .comic-hint {
    text-align: center;
    color: #94a3b8;
  }

  .source-block {
    display: grid;
    gap: 0.45rem;
    margin-top: 0.4rem;
  }

  .source-form {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.6rem;
  }

  .source-form input {
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 12px;
    padding: 0.75rem 0.9rem;
    background: rgba(15, 23, 42, 0.45);
    color: #cbd5e1;
    font: inherit;
  }

  .source-form button {
    border: none;
    border-radius: 999px;
    padding: 0.7rem 1rem;
    background: #38bdf8;
    color: #0f172a;
    font: inherit;
    cursor: pointer;
  }

  .source-form button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .progress-message {
    color: #fecaca;
  }

  small,
  span {
    color: #94a3b8;
  }
</style>
