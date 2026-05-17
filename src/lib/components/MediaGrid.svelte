<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import { ensureMediaPreview, mediaPreviews } from '$lib/stores/mediaPreviewStore';
  import type { MediaItem } from '$lib/types';

  const dispatch = createEventDispatcher<{
    comicFilter: { filter: 'all' | 'unread' | 'reading' | 'completed' };
    import: void;
    search: { query: string };
    select: MediaItem;
    seriesFilter: { filter: string };
  }>();

  export let items: MediaItem[] = [];
  export let busy = false;
  export let errorMessage = '';
  export let runtimeLabel = '';
  export let selectedId: number | null = null;
  export let comicReadFilter: 'all' | 'unread' | 'reading' | 'completed' = 'all';
  export let searchQuery = '';
  export let seriesFilter: string = 'all';
  let uniqueSeriesNames: string[] = [];

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

  function isComic(item: MediaItem) {
    return item.fileType === 'comic';
  }

  function displayName(item: MediaItem) {
    return isComic(item) && item.sourceTitle ? item.sourceTitle : item.fileName;
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
      <p>{runtimeLabel || 'MVPではここに画像・動画・漫画ZIPのサムネイルが並びます。'}</p>
      <p class="subtle-note">取り込んだメディアはアプリ管理フォルダへコピーして保持します。</p>
    </div>
    <button on:click={() => dispatch('import')} disabled={busy}>
      {busy ? '取り込み中...' : 'フォルダを取り込む'}
    </button>
  </div>

  <label class="search-box">
    <span>タイトル検索</span>
    <div class="search-input-row">
      <input
        placeholder="漫画タイトル / ファイル名で検索"
        value={searchQuery}
        on:input={handleSearchInput}
      />
      {#if searchQuery}
        <button type="button" class="search-clear-btn" on:click={() => dispatch('search', { query: '' })}>✕</button>
      {/if}
    </div>
  </label>

  <div class="filter-row">
    <span>漫画の状態</span>
    <div class="filter-buttons">
      <button class:active={comicReadFilter === 'all'} type="button" on:click={() => dispatch('comicFilter', { filter: 'all' })}>すべて</button>
      <button class:active={comicReadFilter === 'unread'} type="button" on:click={() => dispatch('comicFilter', { filter: 'unread' })}>未読</button>
      <button class:active={comicReadFilter === 'reading'} type="button" on:click={() => dispatch('comicFilter', { filter: 'reading' })}>読書中</button>
      <button class:active={comicReadFilter === 'completed'} type="button" on:click={() => dispatch('comicFilter', { filter: 'completed' })}>読了</button>
    </div>
  </div>

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
            {:else}
              画像
            {/if}

            {#if isVideo(item)}
              <span class="video-badge">動画</span>
              {#if preview?.durationSeconds != null}
                <span class="duration-badge">{formatDuration(preview.durationSeconds)}</span>
              {/if}
            {:else if isComic(item)}
              <span class="video-badge comic-badge">漫画</span>
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
            {#if item.seriesName}
              <small class="sub-name">シリーズ: {item.seriesName}</small>
            {/if}
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

  .video-badge {
    position: absolute;
    right: 0.5rem;
    bottom: 0.5rem;
    padding: 0.2rem 0.45rem;
    border-radius: 999px;
    background: rgba(2, 6, 23, 0.75);
    color: #e2e8f0;
    font-size: 0.72rem;
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

  .comic-badge {
    background: rgba(91, 33, 182, 0.78);
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
