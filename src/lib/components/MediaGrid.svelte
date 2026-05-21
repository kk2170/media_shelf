<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import { ensureMediaPreview, mediaPreviews } from '$lib/stores/mediaPreviewStore';
  import type { MediaItem } from '$lib/types';

  const dispatch = createEventDispatcher<{
    comicFilter: { filter: 'all' | 'unread' | 'reading' | 'completed' };
    durationFilter: { filter: 'all' | 'short' | 'medium' | 'long' };
    folderFilter: { filter: string };
    importFile: void;
    import: void;
    orientationFilter: { filter: 'all' | 'portrait' | 'landscape' | 'square' };
    resolutionFilter: { filter: 'all' | 'small' | 'medium' | 'large' };
    search: { query: string };
    select: MediaItem;
    seriesFilter: { filter: string };
    typeFilter: { filter: 'all' | 'image' | 'video' | 'comic' };
  }>();

  export let items: MediaItem[] = [];
  export let busy = false;
  export let errorMessage = '';
  export let runtimeLabel = '';
export let selectedId: number | null = null;
export let comicReadFilter: 'all' | 'unread' | 'reading' | 'completed' = 'all';
export let searchQuery = '';
export let seriesFilter: string = 'all';
export let typeFilter: 'all' | 'image' | 'video' | 'comic' = 'all';
export let orientationFilter: 'all' | 'portrait' | 'landscape' | 'square' = 'all';
export let durationFilter: 'all' | 'short' | 'medium' | 'long' = 'all';
export let resolutionFilter: 'all' | 'small' | 'medium' | 'large' = 'all';
export let folderFilter = 'all';
let uniqueSeriesNames: string[] = [];
let imageCount = 0;
let videoCount = 0;
let comicCount = 0;
let uniqueFolders: string[] = [];

  $: imageCount = items.filter((item) => item.fileType === 'image').length;
  $: videoCount = items.filter((item) => item.fileType === 'video').length;
  $: comicCount = items.filter((item) => item.fileType === 'comic').length;
  $: uniqueFolders = [...new Set(items.map((item) => folderPath(item)).filter(Boolean))].sort();

  $: uniqueSeriesNames = [...new Set(items.map((item) => item.seriesName).filter((name): name is string => name != null))].sort();

  function handleSelect(item: MediaItem) {
    dispatch('select', item);
  }

  function handleSearchInput(event: Event) {
    dispatch('search', { query: (event.currentTarget as HTMLInputElement).value });
  }

  function getPreview(item: MediaItem) {
    return $mediaPreviews[item.id];
  }

  function isVideo(item: MediaItem) {
    return item.fileType === 'video';
  }

  function isImage(item: MediaItem) {
    return item.fileType === 'image';
  }

  function isComic(item: MediaItem) {
    return item.fileType === 'comic';
  }

  function displayName(item: MediaItem) {
    return isComic(item) && item.sourceTitle ? item.sourceTitle : item.fileName;
  }

  function mediaTypeLabel(item: MediaItem) {
    if (item.fileType === 'video') return '動画';
    if (item.fileType === 'comic') return '漫画';
    return '画像';
  }

  function folderPath(item: MediaItem) {
    const rawPath = item.sourcePath ?? item.filePath;
    const normalizedPath = rawPath.replace(/\\/g, '/');
    const lastSlashIndex = normalizedPath.lastIndexOf('/');
    return lastSlashIndex > 0 ? normalizedPath.slice(0, lastSlashIndex) : normalizedPath;
  }

  function folderLabel(path: string) {
    const normalizedPath = path.replace(/\\/g, '/');
    const parts = normalizedPath.split('/').filter(Boolean);
    return parts.at(-1) ?? path;
  }

  function orientationLabel(item: MediaItem) {
    const preview = getPreview(item);
    if (preview?.width == null || preview?.height == null) {
      return '';
    }

    if (preview.width === preview.height) {
      return '正方形';
    }

    return preview.width > preview.height ? '横長' : '縦長';
  }

  function secondaryMeta(item: MediaItem) {
    const preview = getPreview(item);

    if (isVideo(item) && preview?.durationSeconds != null) {
      return `長さ ${formatDuration(preview.durationSeconds)}`;
    }

    if (isComic(item)) {
      const pageCount = preview?.pageCount ?? item.pageCount;
      return pageCount != null ? `${pageCount} ページ` : '漫画ZIP';
    }

    if (preview?.width != null && preview?.height != null) {
      return `${preview.width} × ${preview.height}`;
    }

    return '';
  }

  function comicReadStatus(item: MediaItem): 'unread' | 'reading' | 'completed' | null {
    if (!isComic(item)) {
      return null;
    }

    const pageCount = item.pageCount ?? 0;
    const currentPageIndex = item.currentPageIndex ?? 0;

    if (pageCount <= 0 || currentPageIndex <= 0) {
      return 'unread';
    }

    if (currentPageIndex >= pageCount - 1) {
      return 'completed';
    }

    return 'reading';
  }

  function comicReadStatusLabel(item: MediaItem) {
    const status = comicReadStatus(item);
    if (status === 'completed') return '読了';
    if (status === 'reading') return '読書中';
    if (status === 'unread') return '未読';
    return '';
  }

  function formatDuration(value: number | null | undefined) {
    if (value == null || Number.isNaN(value)) {
      return '';
    }

    const totalSeconds = Math.max(0, Math.round(value));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    return hours > 0
      ? `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
      : `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }

  $: for (const item of items) {
    ensureMediaPreview(item);
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <h2>ライブラリ</h2>
      <p>{runtimeLabel || '画像・動画・漫画をまとめて閲覧・整理できます。'}</p>
      <p class="subtle-note">取り込んだメディアはアプリ管理フォルダへコピーして保持します。</p>
    </div>
    <div class="header-actions">
      <button on:click={() => dispatch('importFile')} disabled={busy}>
        {busy ? '取り込み中...' : 'ファイルを取り込む'}
      </button>
      <button on:click={() => dispatch('import')} disabled={busy}>
        {busy ? '取り込み中...' : 'フォルダを取り込む'}
      </button>
    </div>
  </div>

  <div class="library-stats">
    <div class="stat-chip">
      <strong>{imageCount}</strong>
      <span>画像</span>
    </div>
    <div class="stat-chip">
      <strong>{videoCount}</strong>
      <span>動画</span>
    </div>
    <div class="stat-chip">
      <strong>{comicCount}</strong>
      <span>漫画</span>
    </div>
  </div>

  <label class="search-box">
    <span>検索</span>
    <div class="search-input-row">
      <input
        placeholder="タイトル / ファイル名 / シリーズ / パスで検索"
        value={searchQuery}
        on:input={handleSearchInput}
      />
      {#if searchQuery}
        <button type="button" class="search-clear-btn" on:click={() => dispatch('search', { query: '' })}>✕</button>
      {/if}
    </div>
  </label>

  <div class="filter-row">
    <span>メディア種別</span>
    <div class="filter-buttons">
      <button class:active={typeFilter === 'all'} type="button" on:click={() => dispatch('typeFilter', { filter: 'all' })}>すべて</button>
      <button class:active={typeFilter === 'image'} type="button" on:click={() => dispatch('typeFilter', { filter: 'image' })}>画像</button>
      <button class:active={typeFilter === 'video'} type="button" on:click={() => dispatch('typeFilter', { filter: 'video' })}>動画</button>
      <button class:active={typeFilter === 'comic'} type="button" on:click={() => dispatch('typeFilter', { filter: 'comic' })}>漫画</button>
    </div>
  </div>

  {#if videoCount > 0}
    <div class="filter-row">
      <span>動画の長さ</span>
      <div class="filter-buttons">
        <button class:active={durationFilter === 'all'} type="button" on:click={() => dispatch('durationFilter', { filter: 'all' })}>すべて</button>
        <button class:active={durationFilter === 'short'} type="button" on:click={() => dispatch('durationFilter', { filter: 'short' })}>短編</button>
        <button class:active={durationFilter === 'medium'} type="button" on:click={() => dispatch('durationFilter', { filter: 'medium' })}>中編</button>
        <button class:active={durationFilter === 'long'} type="button" on:click={() => dispatch('durationFilter', { filter: 'long' })}>長編</button>
      </div>
    </div>
  {/if}

  {#if imageCount > 0}
    <div class="filter-row">
      <span>画像の解像度</span>
      <div class="filter-buttons">
        <button class:active={resolutionFilter === 'all'} type="button" on:click={() => dispatch('resolutionFilter', { filter: 'all' })}>すべて</button>
        <button class:active={resolutionFilter === 'small'} type="button" on:click={() => dispatch('resolutionFilter', { filter: 'small' })}>低</button>
        <button class:active={resolutionFilter === 'medium'} type="button" on:click={() => dispatch('resolutionFilter', { filter: 'medium' })}>中</button>
        <button class:active={resolutionFilter === 'large'} type="button" on:click={() => dispatch('resolutionFilter', { filter: 'large' })}>高</button>
      </div>
    </div>
  {/if}

  <div class="filter-row">
    <span>向き</span>
    <div class="filter-buttons">
      <button class:active={orientationFilter === 'all'} type="button" on:click={() => dispatch('orientationFilter', { filter: 'all' })}>すべて</button>
      <button class:active={orientationFilter === 'portrait'} type="button" on:click={() => dispatch('orientationFilter', { filter: 'portrait' })}>縦長</button>
      <button class:active={orientationFilter === 'landscape'} type="button" on:click={() => dispatch('orientationFilter', { filter: 'landscape' })}>横長</button>
      <button class:active={orientationFilter === 'square'} type="button" on:click={() => dispatch('orientationFilter', { filter: 'square' })}>正方形</button>
    </div>
  </div>

  {#if uniqueFolders.length > 0}
    <label class="filter-row folder-filter">
      <span>フォルダ</span>
      <select value={folderFilter} on:change={(event) => dispatch('folderFilter', { filter: (event.currentTarget as HTMLSelectElement).value })}>
        <option value="all">すべてのフォルダ</option>
        {#each uniqueFolders as path}
          <option value={path}>{folderLabel(path)}</option>
        {/each}
      </select>
    </label>
  {/if}

  {#if comicCount > 0}
    <div class="filter-row">
      <span>漫画の読書状態</span>
      <div class="filter-buttons">
        <button class:active={comicReadFilter === 'all'} type="button" on:click={() => dispatch('comicFilter', { filter: 'all' })}>すべて</button>
        <button class:active={comicReadFilter === 'unread'} type="button" on:click={() => dispatch('comicFilter', { filter: 'unread' })}>未読</button>
        <button class:active={comicReadFilter === 'reading'} type="button" on:click={() => dispatch('comicFilter', { filter: 'reading' })}>読書中</button>
        <button class:active={comicReadFilter === 'completed'} type="button" on:click={() => dispatch('comicFilter', { filter: 'completed' })}>読了</button>
      </div>
    </div>
  {/if}

  {#if uniqueSeriesNames.length > 0}
    <div class="filter-row">
      <span>シリーズ</span>
      <div class="filter-buttons">
        <button class:active={seriesFilter === 'all'} type="button" on:click={() => dispatch('seriesFilter', { filter: 'all' })}>すべて</button>
        {#each uniqueSeriesNames as name}
          <button class:active={seriesFilter === name} type="button" on:click={() => dispatch('seriesFilter', { filter: name })}>{name}</button>
        {/each}
      </div>
    </div>
  {/if}

  {#if errorMessage}
    <div class="error-state">{errorMessage}</div>
  {/if}

  {#if items.length === 0}
    <div class="empty-state">
      <strong>まだメディアが読み込まれていません。</strong>
      <p>ローカルフォルダを選ぶと、画像・動画・漫画ZIPをスキャンして SQLite に登録します。</p>
    </div>
  {:else}
    <div class="grid">
      {#each items as item}
        {@const preview = getPreview(item)}
        <button
          type="button"
          class:selected={item.id === selectedId}
          class="tile"
          on:click={() => handleSelect(item)}
        >
          <div class="thumb">
            {#if preview?.thumbnailUrl}
              <img alt={displayName(item)} src={preview.thumbnailUrl} loading="lazy" />
            {:else if preview?.assetUrl && item.fileType === 'image'}
              <img alt={displayName(item)} src={preview.assetUrl} loading="lazy" />
            {:else if preview?.status === 'loading'}
              <span class="loading-label">生成中...</span>
            {:else if preview?.errorMessage}
              <span class="loading-label">{preview.errorMessage}</span>
            {:else if isComic(item)}
              漫画ZIP
            {:else if isVideo(item)}
              動画
            {:else}
              画像
            {/if}

            <span class={`media-badge ${item.fileType}`}>{mediaTypeLabel(item)}</span>

            {#if isVideo(item)}
              {#if preview?.durationSeconds != null}
                <span class="duration-badge">{formatDuration(preview.durationSeconds)}</span>
              {/if}
            {:else if isComic(item)}
              {#if (preview?.pageCount ?? item.pageCount) != null}
                <span class="duration-badge">{preview?.pageCount ?? item.pageCount}p</span>
              {/if}
              {#if comicReadStatus(item)}
                <span class={`status-badge ${comicReadStatus(item)}`}>{comicReadStatusLabel(item)}</span>
              {/if}
            {/if}
          </div>
          <div class="tile-meta">
            <strong>{displayName(item)}</strong>
            <div class="meta-line primary-meta">
              <small>{mediaTypeLabel(item)}</small>
              {#if secondaryMeta(item)}
                <small>{secondaryMeta(item)}</small>
              {/if}
              {#if orientationLabel(item)}
                <small>{orientationLabel(item)}</small>
              {/if}
            </div>
            {#if item.seriesName}
              <small class="sub-name">シリーズ: {item.seriesName}</small>
            {/if}
            <small class="sub-name">フォルダ: {folderLabel(folderPath(item))}</small>
            {#if isComic(item) && item.sourceTitle}
              <small class="sub-name">元ファイル: {item.fileName}</small>
            {/if}
            {#if preview?.width != null && preview?.height != null}
              <small>{preview.width} × {preview.height}</small>
            {/if}
            <span>{item.filePath}</span>
          </div>
        </button>
      {/each}
    </div>
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

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
  }

  .header-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    justify-content: flex-end;
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  p {
    margin: 0.35rem 0 0;
    color: #94a3b8;
  }

  .subtle-note {
    font-size: 0.82rem;
  }

  .library-stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.75rem;
  }

  .stat-chip {
    display: grid;
    gap: 0.15rem;
    padding: 0.8rem 0.95rem;
    border-radius: 14px;
    background: rgba(30, 41, 59, 0.72);
    border: 1px solid rgba(148, 163, 184, 0.12);
  }

  .stat-chip strong {
    font-size: 1.1rem;
    color: #f8fafc;
  }

  .stat-chip span {
    font-size: 0.82rem;
    color: #94a3b8;
  }

  button {
    border: none;
    border-radius: 999px;
    padding: 0.7rem 1rem;
    font: inherit;
    color: #0f172a;
    background: #38bdf8;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.55;
    cursor: wait;
  }

  .search-box {
    display: grid;
    gap: 0.4rem;
  }

  .search-box span {
    color: #cbd5e1;
    font-size: 0.88rem;
  }

  .search-box input {
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 12px;
    padding: 0.8rem 0.9rem;
    background: rgba(15, 23, 42, 0.45);
    color: #cbd5e1;
    font: inherit;
  }

  .search-input-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    position: relative;
  }

  .search-input-row input {
    flex: 1;
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 12px;
    padding: 0.8rem 0.9rem;
    background: rgba(15, 23, 42, 0.45);
    color: #cbd5e1;
    font: inherit;
  }

  .search-clear-btn {
    position: absolute;
    right: 0.5rem;
    width: 1.6rem;
    height: 1.6rem;
    display: grid;
    place-items: center;
    padding: 0;
    border-radius: 50%;
    background: rgba(148, 163, 184, 0.25);
    color: #cbd5e1;
    font-size: 0.75rem;
    line-height: 1;
  }

  .search-clear-btn:hover {
    background: rgba(148, 163, 184, 0.45);
  }

  .filter-row {
    display: grid;
    gap: 0.4rem;
  }

  .filter-row span {
    color: #cbd5e1;
    font-size: 0.88rem;
  }

  .folder-filter select {
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 12px;
    padding: 0.8rem 0.9rem;
    background: rgba(15, 23, 42, 0.45);
    color: #cbd5e1;
    font: inherit;
  }

  .filter-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .filter-buttons button {
    background: rgba(15, 23, 42, 0.45);
    color: #cbd5e1;
    border: 1px solid rgba(148, 163, 184, 0.2);
  }

  .filter-buttons button.active {
    background: rgba(14, 165, 233, 0.22);
    border-color: rgba(56, 189, 248, 0.8);
    color: #e0f2fe;
  }

  .empty-state {
    display: grid;
    gap: 0.5rem;
    padding: 1.5rem;
    border-radius: 16px;
    background: rgba(30, 41, 59, 0.72);
    color: #cbd5e1;
  }

  .error-state {
    padding: 0.9rem 1rem;
    border-radius: 14px;
    background: rgba(220, 38, 38, 0.16);
    color: #fecaca;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 1rem;
  }

  .tile {
    display: grid;
    gap: 0.75rem;
    border: 1px solid transparent;
    padding: 0.75rem;
    border-radius: 16px;
    background: rgba(30, 41, 59, 0.72);
    cursor: pointer;
    color: inherit;
    font: inherit;
    text-align: left;
    transition:
      transform 0.15s ease,
      border-color 0.15s ease,
      background 0.15s ease;
  }

  .tile:hover {
    transform: translateY(-1px);
    border-color: rgba(56, 189, 248, 0.35);
  }

  .tile.selected {
    border-color: rgba(56, 189, 248, 0.7);
    background: rgba(15, 23, 42, 0.9);
  }

  .thumb {
    display: grid;
    place-items: center;
    position: relative;
    overflow: hidden;
    min-height: 120px;
    border-radius: 12px;
    background: linear-gradient(135deg, rgba(56, 189, 248, 0.2), rgba(168, 85, 247, 0.25));
    color: #e2e8f0;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    min-height: 120px;
    object-fit: cover;
    display: block;
  }

  .loading-label {
    padding: 0.5rem 0.75rem;
    text-align: center;
    font-size: 0.8rem;
    color: #cbd5e1;
  }

  .media-badge {
    position: absolute;
    right: 0.5rem;
    top: 0.5rem;
    padding: 0.2rem 0.45rem;
    border-radius: 999px;
    background: rgba(2, 6, 23, 0.75);
    color: #e2e8f0;
    font-size: 0.72rem;
  }

  .media-badge.video {
    background: rgba(3, 105, 161, 0.85);
  }

  .media-badge.image {
    background: rgba(22, 101, 52, 0.85);
  }

  .media-badge.comic {
    background: rgba(91, 33, 182, 0.85);
  }

  .duration-badge {
    position: absolute;
    left: 0.5rem;
    bottom: 0.5rem;
    padding: 0.2rem 0.45rem;
    border-radius: 999px;
    background: rgba(2, 6, 23, 0.75);
    color: #e2e8f0;
    font-size: 0.72rem;
  }

  .status-badge {
    position: absolute;
    left: 0.5rem;
    top: 0.5rem;
    padding: 0.2rem 0.45rem;
    border-radius: 999px;
    color: #e2e8f0;
    font-size: 0.72rem;
    background: rgba(30, 41, 59, 0.9);
  }

  .status-badge.unread {
    background: rgba(71, 85, 105, 0.9);
  }

  .status-badge.reading {
    background: rgba(2, 132, 199, 0.9);
  }

  .status-badge.completed {
    background: rgba(22, 163, 74, 0.9);
  }

  .tile-meta {
    display: grid;
    gap: 0.25rem;
  }

  .tile-meta small {
    color: #cbd5e1;
    font-size: 0.78rem;
  }

  .meta-line {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.5rem;
  }

  .primary-meta small {
    color: #cbd5e1;
  }

  .tile-meta .sub-name {
    color: #94a3b8;
  }

  .tile-meta span {
    color: #94a3b8;
    font-size: 0.85rem;
    word-break: break-all;
  }

  .tile:focus-visible {
    border-color: rgba(125, 211, 252, 0.85);
  }
</style>
