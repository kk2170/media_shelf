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

  function mediaTypeLabel(currentItem: MediaItem | null) {
    if (currentItem?.fileType === 'video') return '動画';
    if (currentItem?.fileType === 'comic') return '漫画';
    return '画像';
  }

  function orientationLabel(currentItem: MediaItem | null) {
    const preview = getPreview(currentItem);
    if (preview?.width == null || preview?.height == null) {
      return '不明';
    }

    if (preview.width === preview.height) {
      return '正方形';
    }

    return preview.width > preview.height ? '横長' : '縦長';
  }

  function sourceDirectory(currentItem: MediaItem | null) {
    const path = currentItem?.sourcePath ?? currentItem?.filePath;
    if (!path) {
      return '不明';
    }

    const normalized = path.replace(/\\/g, '/');
    const lastSlashIndex = normalized.lastIndexOf('/');
    return lastSlashIndex > 0 ? normalized.slice(0, lastSlashIndex) : normalized;
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

      <div class="viewer-heading">
        <div>
          <strong>{displayName(item)}</strong>
          {#if isComic(item) && item.sourceTitle}
            <small>元ファイル名: {item.fileName}</small>
          {/if}
        </div>
        <span class={`type-pill ${item.fileType}`}>{mediaTypeLabel(item)}</span>
      </div>

      <div class="info-section">
        <h3>基本情報</h3>
        <div class="fact-grid">
          <div class="fact-card">
            <small>種別</small>
            <strong>{mediaTypeLabel(item)}</strong>
          </div>
          <div class="fact-card">
            <small>サイズ</small>
            <strong>{formatFileSize(item.fileSize)}</strong>
          </div>
          <div class="fact-card">
            <small>管理方式</small>
            <strong>{item.sourcePath ? 'コピー管理' : '外部参照（旧データ）'}</strong>
          </div>
          <div class="fact-card">
            <small>保存元フォルダ</small>
            <strong>{sourceDirectory(item)}</strong>
          </div>
        </div>
      </div>

      <div class="info-section">
        <h3>メディア情報</h3>
        <div class="fact-grid">
          <div class="fact-card">
            <small>向き</small>
            <strong>{orientationLabel(item)}</strong>
          </div>
          {#if preview?.width != null && preview?.height != null}
            <div class="fact-card">
              <small>解像度</small>
              <strong>{preview.width} × {preview.height}</strong>
            </div>
          {/if}
          {#if item.fileType === 'video'}
            <div class="fact-card">
              <small>長さ</small>
              <strong>{formatDuration(preview?.durationSeconds)}</strong>
            </div>
          {/if}
          {#if isComic(item)}
            <div class="fact-card">
              <small>読書状態</small>
              <strong>{comicReadStatusLabel(item)}</strong>
            </div>
            <div class="fact-card">
              <small>ページ数</small>
              <strong>{preview?.pageCount ?? item.pageCount ?? '不明'}</strong>
            </div>
          {/if}
        </div>
      </div>

      <div class="info-section">
        <h3>履歴</h3>
        <div class="fact-grid">
          <div class="fact-card">
            <small>取り込み時刻</small>
            <strong>{formatImportedAt(item.importedAt)}</strong>
          </div>
          {#if item.lastViewedAt}
            <div class="fact-card">
              <small>最終閲覧</small>
              <strong>{formatImportedAt(item.lastViewedAt)}</strong>
            </div>
          {/if}
          {#if item.completedAt}
            <div class="fact-card">
              <small>読了時刻</small>
              <strong>{formatImportedAt(item.completedAt)}</strong>
            </div>
          {/if}
        </div>
      </div>

      {#if isComic(item)}
        <div class="info-section">
          <h3>漫画向け情報</h3>
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
    gap: 0.85rem;
    padding: 1.2rem;
    border-radius: 16px;
    background: rgba(30, 41, 59, 0.72);
  }

  .viewer-heading {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .viewer-heading strong {
    display: block;
  }

  .type-pill {
    display: inline-flex;
    align-items: center;
    padding: 0.3rem 0.65rem;
    border-radius: 999px;
    font-size: 0.8rem;
    color: #e2e8f0;
    background: rgba(30, 41, 59, 0.95);
  }

  .type-pill.image {
    background: rgba(22, 101, 52, 0.9);
  }

  .type-pill.video {
    background: rgba(3, 105, 161, 0.9);
  }

  .type-pill.comic {
    background: rgba(91, 33, 182, 0.9);
  }

  .info-section {
    display: grid;
    gap: 0.6rem;
  }

  .info-section h3 {
    margin: 0;
    font-size: 0.92rem;
    color: #e2e8f0;
  }

  .fact-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.65rem;
  }

  .fact-card {
    display: grid;
    gap: 0.2rem;
    padding: 0.8rem 0.9rem;
    border-radius: 14px;
    background: rgba(15, 23, 42, 0.45);
    border: 1px solid rgba(148, 163, 184, 0.14);
  }

  .fact-card small {
    color: #94a3b8;
  }

  .fact-card strong {
    color: #f8fafc;
    font-size: 0.92rem;
    word-break: break-word;
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
