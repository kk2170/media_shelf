<script lang="ts">
  import { onMount } from 'svelte';

  import MediaGrid from '$lib/components/MediaGrid.svelte';
  import MediaViewer from '$lib/components/MediaViewer.svelte';
  import TagEditor from '$lib/components/TagEditor.svelte';
  import TagFilterPanel from '$lib/components/TagFilterPanel.svelte';
  import { mediaPreviews } from '$lib/stores/mediaPreviewStore';
  import type { MediaItem } from '$lib/types';
  import {
    createTag,
    createTagCategory,
    importMediaFile,
    importMediaFolder,
    isTauriApp,
    loadLibrarySnapshot,
    pickImportFile,
    pickImportDirectory,
    setComicProgress,
    setMediaSeries,
    setMediaSourceFromUrl,
    setMediaTags,
    touchLastViewed
  } from '$lib/tauri';
  import {
    mediaItems,
    selectedMedia,
    selectedMediaId,
    setMediaItems,
    setSelectedMediaId
  } from '$lib/stores/mediaStore';
  import { setTagData, tagCategories, tags } from '$lib/stores/tagStore';

  let busy = false;
  let errorMessage = '';
  let comicProgressMessage = '';
  let runtimeLabel = 'Tauriデスクトップ起動時にライブラリを読み込みます。';
  let lastImportSummary = '未実行';
  let searchQuery = '';
  let comicReadFilter: 'all' | 'unread' | 'reading' | 'completed' = 'all';
  let seriesFilter: string = 'all';
  let typeFilter: 'all' | 'image' | 'video' | 'comic' = 'all';
  let orientationFilter: 'all' | 'portrait' | 'landscape' | 'square' = 'all';
  let durationFilter: 'all' | 'short' | 'medium' | 'long' = 'all';
  let resolutionFilter: 'all' | 'small' | 'medium' | 'large' = 'all';
  let folderFilter = 'all';
  let selectedFilterTagIds: number[] = [];
  let filteredMediaItems: MediaItem[] = [];
  let selectedMediaTagIds: number[] = [];
  let comicProgressSaveTimer: ReturnType<typeof setTimeout> | null = null;
  let comicProgressSaveSequence = 0;

  function displayName(item: MediaItem) {
    return item.fileType === 'comic' && item.sourceTitle ? item.sourceTitle : item.fileName;
  }

  function comicReadStatus(item: MediaItem): 'other' | 'unread' | 'reading' | 'completed' {
    if (item.fileType !== 'comic') {
      return 'other';
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

  function itemSearchText(item: MediaItem) {
    return [
      displayName(item),
      item.fileName,
      item.seriesName ?? '',
      item.sourcePath ?? '',
      item.filePath,
      item.mimeType ?? '',
      item.fileType
    ]
      .join(' ')
      .toLocaleLowerCase('ja-JP');
  }

  function itemOrientation(item: MediaItem): 'portrait' | 'landscape' | 'square' | 'unknown' {
    const preview = $mediaPreviews[item.id];
    if (preview?.width == null || preview?.height == null) {
      return 'unknown';
    }

    if (preview.width === preview.height) {
      return 'square';
    }

    return preview.width > preview.height ? 'landscape' : 'portrait';
  }

  function itemDurationBucket(item: MediaItem): 'short' | 'medium' | 'long' | 'unknown' {
    const durationSeconds = $mediaPreviews[item.id]?.durationSeconds;
    if (durationSeconds == null || !Number.isFinite(durationSeconds)) {
      return 'unknown';
    }

    if (durationSeconds < 5 * 60) {
      return 'short';
    }

    if (durationSeconds < 20 * 60) {
      return 'medium';
    }

    return 'long';
  }

  function itemResolutionBucket(item: MediaItem): 'small' | 'medium' | 'large' | 'unknown' {
    const preview = $mediaPreviews[item.id];
    if (preview?.width == null || preview?.height == null) {
      return 'unknown';
    }

    const pixels = preview.width * preview.height;
    if (pixels < 1280 * 720) {
      return 'small';
    }

    if (pixels < 2560 * 1440) {
      return 'medium';
    }

    return 'large';
  }

  function itemFolderPath(item: MediaItem) {
    const rawPath = item.sourcePath ?? item.filePath;
    const normalizedPath = rawPath.replace(/\\/g, '/');
    const lastSlashIndex = normalizedPath.lastIndexOf('/');
    return lastSlashIndex > 0 ? normalizedPath.slice(0, lastSlashIndex) : normalizedPath;
  }

  $: filteredMediaItems = $mediaItems.filter((item) =>
    selectedFilterTagIds.every((tagId) => item.tagIds.includes(tagId)) &&
    itemSearchText(item).includes(searchQuery.trim().toLocaleLowerCase('ja-JP')) &&
    (typeFilter === 'all' || item.fileType === typeFilter) &&
    (orientationFilter === 'all' || itemOrientation(item) === orientationFilter) &&
    (durationFilter === 'all' || (item.fileType === 'video' && itemDurationBucket(item) === durationFilter)) &&
    (resolutionFilter === 'all' || (item.fileType === 'image' && itemResolutionBucket(item) === resolutionFilter)) &&
    (folderFilter === 'all' || itemFolderPath(item) === folderFilter) &&
    (comicReadFilter === 'all' || comicReadStatus(item) === comicReadFilter) &&
    (seriesFilter === 'all' || item.seriesName === seriesFilter)
  );

  $: selectedMediaTagIds = $selectedMedia?.tagIds ?? [];

  $: if ($selectedMedia?.fileType !== 'comic') {
    comicProgressMessage = '';
  }

  $: if (
    $selectedMediaId !== null &&
    !filteredMediaItems.some((item) => item.id === $selectedMediaId)
  ) {
    setSelectedMediaId(filteredMediaItems[0]?.id ?? null);
  }

  onMount(async () => {
    if (!isTauriApp()) {
      runtimeLabel = 'ブラウザプレビュー中です。フォルダ取り込みは Tauri デスクトップ起動時に有効になります。';
      return;
    }

    await refreshLibrarySnapshot();
  });

  async function refreshLibrarySnapshot() {
    busy = true;
    errorMessage = '';

    try {
      const snapshot = await loadLibrarySnapshot();
      setMediaItems(snapshot.mediaItems);
      setTagData(snapshot.tagCategories, snapshot.tags);
      runtimeLabel = `保存済みメディア ${snapshot.mediaItems.length} 件 / タグ ${snapshot.tags.length} 件を読み込みました。`;
      lastImportSummary =
        snapshot.mediaItems.length > 0 ? '前回までの取り込み結果を表示中' : 'まだメディアはありません';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'メディア一覧の読み込みに失敗しました。';
    } finally {
      busy = false;
    }
  }

  async function handleImport() {
    if (!isTauriApp()) {
      errorMessage = 'フォルダ取り込みは Tauri デスクトップアプリで実行してください。';
      return;
    }

    errorMessage = '';

    const folderPath = await pickImportDirectory();
    if (!folderPath) {
      runtimeLabel = 'フォルダ選択をキャンセルしました。';
      return;
    }

    busy = true;
    runtimeLabel = `スキャン中: ${folderPath}`;

    try {
      const result = await importMediaFolder(folderPath);
      setMediaItems(result.mediaItems);
      runtimeLabel = `${result.folderPath} をスキャンしました。`;
      lastImportSummary = `${result.totalScanned} 件検出 / ${result.totalImported} 件を新規登録`;
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'フォルダ取り込みに失敗しました。';
    } finally {
      busy = false;
    }
  }

  async function handleImportFile() {
    if (!isTauriApp()) {
      errorMessage = 'ファイル取り込みは Tauri デスクトップアプリで実行してください。';
      return;
    }

    errorMessage = '';

    const filePath = await pickImportFile();
    if (!filePath) {
      runtimeLabel = 'ファイル選択をキャンセルしました。';
      return;
    }

    busy = true;
    runtimeLabel = `取り込み中: ${filePath}`;

    try {
      const result = await importMediaFile(filePath);
      setMediaItems(result.mediaItems);
      runtimeLabel = `${filePath} を取り込みました。`;
      lastImportSummary = result.totalImported > 0 ? '1 件を新規登録' : 'すでに登録済みのため追加はありません';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'ファイル取り込みに失敗しました。';
    } finally {
      busy = false;
    }
  }

  async function handleCreateCategory(event: CustomEvent<{ name: string; color: string | null }>) {
    if (!isTauriApp()) return;

    busy = true;
    errorMessage = '';

    try {
      await createTagCategory(event.detail.name, event.detail.color);
      await refreshLibrarySnapshot();
      runtimeLabel = 'タグカテゴリを作成しました。';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'タグカテゴリの作成に失敗しました。';
      busy = false;
    }
  }

  async function handleCreateTag(
    event: CustomEvent<{ categoryId: number; name: string; description: string | null; color: string | null }>
  ) {
    if (!isTauriApp()) return;

    busy = true;
    errorMessage = '';

    try {
      const { categoryId, name, description, color } = event.detail;
      await createTag(categoryId, name, description, color);
      await refreshLibrarySnapshot();
      runtimeLabel = 'タグを作成しました。';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'タグの作成に失敗しました。';
      busy = false;
    }
  }

  async function handleUpdateMediaTags(event: CustomEvent<{ tagIds: number[] }>) {
    if (!isTauriApp() || !$selectedMediaId) {
      return;
    }

    busy = true;
    errorMessage = '';

    try {
      const updatedItem = await setMediaTags($selectedMediaId, event.detail.tagIds);
      const nextItems = $mediaItems.map((item) => (item.id === updatedItem.id ? updatedItem : item));
      setMediaItems(nextItems);
      runtimeLabel = 'メディアのタグを更新しました。';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'メディアのタグ更新に失敗しました。';
    } finally {
      busy = false;
    }
  }

  async function handleResolveSource(event: CustomEvent<{ sourceUrl: string }>) {
    if (!isTauriApp() || !$selectedMediaId) {
      return;
    }

    busy = true;
    errorMessage = '';

    try {
      const result = await setMediaSourceFromUrl($selectedMediaId, event.detail.sourceUrl);
      const nextItems = $mediaItems.map((item) => (item.id === result.mediaItem.id ? result.mediaItem : item));
      setMediaItems(nextItems);
      runtimeLabel = `タイトルを取得しました: ${result.title}`;
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'URLからタイトルを取得できませんでした。';
    } finally {
      busy = false;
    }
  }

  async function handleComicProgress(event: CustomEvent<{ mediaId: number; pageIndex: number }>) {
    if (!isTauriApp()) {
      return;
    }

    comicProgressMessage = '';

    if (comicProgressSaveTimer) {
      clearTimeout(comicProgressSaveTimer);
    }

    const { mediaId, pageIndex } = event.detail;
    const requestSequence = ++comicProgressSaveSequence;

    comicProgressSaveTimer = setTimeout(async () => {
      try {
        const updatedItem = await setComicProgress(mediaId, pageIndex);
        if (requestSequence !== comicProgressSaveSequence) {
          return;
        }

        const nextItems = $mediaItems.map((item) => (item.id === updatedItem.id ? updatedItem : item));
        setMediaItems(nextItems);
      } catch (error) {
        if (requestSequence !== comicProgressSaveSequence) {
          return;
        }

        comicProgressMessage = error instanceof Error ? error.message : '読了位置の保存に失敗しました。';
      }
    }, 400);
  }

  function handleToggleFilterTag(event: CustomEvent<{ tagId: number }>) {
    const { tagId } = event.detail;
    selectedFilterTagIds = selectedFilterTagIds.includes(tagId)
      ? selectedFilterTagIds.filter((currentTagId) => currentTagId !== tagId)
      : [...selectedFilterTagIds, tagId];
  }

  function handleClearFilters() {
    selectedFilterTagIds = [];
  }

  function handleSearch(event: CustomEvent<{ query: string }>) {
    searchQuery = event.detail.query;
  }

  function handleComicFilter(event: CustomEvent<{ filter: 'all' | 'unread' | 'reading' | 'completed' }>) {
    comicReadFilter = event.detail.filter;
  }

  function handleSeriesFilter(event: CustomEvent<{ filter: string }>) {
    seriesFilter = event.detail.filter;
  }

  function handleTypeFilter(event: CustomEvent<{ filter: 'all' | 'image' | 'video' | 'comic' }>) {
    typeFilter = event.detail.filter;
  }

  function handleOrientationFilter(event: CustomEvent<{ filter: 'all' | 'portrait' | 'landscape' | 'square' }>) {
    orientationFilter = event.detail.filter;
  }

  function handleDurationFilter(event: CustomEvent<{ filter: 'all' | 'short' | 'medium' | 'long' }>) {
    durationFilter = event.detail.filter;
  }

  function handleResolutionFilter(event: CustomEvent<{ filter: 'all' | 'small' | 'medium' | 'large' }>) {
    resolutionFilter = event.detail.filter;
  }

  function handleFolderFilter(event: CustomEvent<{ filter: string }>) {
    folderFilter = event.detail.filter;
  }

  async function handleUpdateSeries(event: CustomEvent<{ mediaId: number; seriesName: string }>) {
    if (!isTauriApp()) {
      return;
    }

    busy = true;
    errorMessage = '';

    try {
      const updatedItem = await setMediaSeries(event.detail.mediaId, event.detail.seriesName);
      const nextItems = $mediaItems.map((item) => (item.id === updatedItem.id ? updatedItem : item));
      setMediaItems(nextItems);
      runtimeLabel = 'シリーズ名を保存しました。';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : 'シリーズ名の保存に失敗しました。';
    } finally {
      busy = false;
    }
  }

  function handleSelectItem(event: CustomEvent<MediaItem>) {
    const item = event.detail;
    setSelectedMediaId(item.id);

    if (!isTauriApp()) {
      return;
    }

    touchLastViewed(item.id)
      .then((updatedItem) => {
        const nextItems = $mediaItems.map((i) => (i.id === updatedItem.id ? updatedItem : i));
        setMediaItems(nextItems);
      })
      .catch((error) => {
        errorMessage = error instanceof Error ? error.message : '最終閲覧時刻の更新に失敗しました。';
      });
  }
</script>

<svelte:head>
  <title>Media Shelf</title>
  <meta
    name="description"
    content="ローカル画像・動画・漫画ZIPをカテゴリ付きタグで管理するデスクトップアプリのMVP土台"
  />
</svelte:head>

<div class="page-shell">
  <header class="hero">
    <div>
      <span class="eyebrow">MVP foundation</span>
      <h1>Media Shelf</h1>
      <p>
        ローカルメディアを「人物」「作品」「用途」などの意味付きタグで管理するための
        Tauri + Svelte + SQLite のライブラリアプリです。画像・動画・漫画を同じ棚で整理できます。
      </p>
    </div>

    <div class="status-card">
      <strong>今回の着手範囲</strong>
      <ul>
        <li>SQLiteスキーマ追加</li>
        <li>取り込み時にアプリ管理フォルダへコピー保存</li>
        <li>フォルダ選択 + 画像 / 動画 / 漫画スキャン</li>
        <li>画像表示・動画再生・漫画ZIPページ送り閲覧</li>
        <li>SQLite保存済みライブラリの一覧表示</li>
        <li>タグカテゴリ作成・タグ作成・タグ付け</li>
        <li>タグ・種別・向き・シリーズ検索</li>
      </ul>
      <p class="summary">{lastImportSummary}</p>
    </div>
  </header>

  <main class="workspace">
    <div class="sidebar">
      <TagFilterPanel
        categories={$tagCategories}
        tags={$tags}
        selectedTagIds={selectedFilterTagIds}
        on:toggle={handleToggleFilterTag}
        on:clear={handleClearFilters}
      />
      <TagEditor
        categories={$tagCategories}
        tags={$tags}
        selectedMediaId={$selectedMediaId}
        selectedTagIds={selectedMediaTagIds}
        busy={busy}
        on:createCategory={handleCreateCategory}
        on:createTag={handleCreateTag}
        on:updateMediaTags={handleUpdateMediaTags}
      />
    </div>

    <div class="content">
      <MediaGrid
        items={filteredMediaItems}
        busy={busy}
        errorMessage={errorMessage}
        runtimeLabel={runtimeLabel}
        selectedId={$selectedMediaId}
        comicReadFilter={comicReadFilter}
        durationFilter={durationFilter}
        folderFilter={folderFilter}
        typeFilter={typeFilter}
        orientationFilter={orientationFilter}
        resolutionFilter={resolutionFilter}
        seriesFilter={seriesFilter}
        searchQuery={searchQuery}
        on:comicFilter={handleComicFilter}
        on:durationFilter={handleDurationFilter}
        on:folderFilter={handleFolderFilter}
        on:import={handleImport}
        on:importFile={handleImportFile}
        on:orientationFilter={handleOrientationFilter}
        on:resolutionFilter={handleResolutionFilter}
        on:search={handleSearch}
        on:select={handleSelectItem}
        on:seriesFilter={handleSeriesFilter}
        on:typeFilter={handleTypeFilter}
      />
      <MediaViewer
        item={$selectedMedia}
        busy={busy}
        progressMessage={comicProgressMessage}
        on:comicProgress={handleComicProgress}
        on:resolveSource={handleResolveSource}
        on:updateSeries={handleUpdateSeries}
      />
    </div>
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    font-family: Inter, "Noto Sans JP", system-ui, sans-serif;
    background:
      radial-gradient(circle at top, rgba(56, 189, 248, 0.18), transparent 30%),
      linear-gradient(180deg, #020617 0%, #0f172a 100%);
    color: #e2e8f0;
  }

  .page-shell {
    min-height: 100vh;
    padding: 2rem;
    box-sizing: border-box;
    display: grid;
    gap: 1.5rem;
  }

  .hero {
    display: grid;
    grid-template-columns: minmax(0, 1.8fr) minmax(280px, 0.9fr);
    gap: 1rem;
    align-items: stretch;
  }

  .eyebrow {
    display: inline-flex;
    margin-bottom: 0.65rem;
    padding: 0.35rem 0.65rem;
    border-radius: 999px;
    background: rgba(56, 189, 248, 0.12);
    color: #7dd3fc;
    font-size: 0.85rem;
  }

  h1 {
    margin: 0;
    font-size: clamp(2rem, 4vw, 3.2rem);
  }

  .hero p {
    max-width: 60rem;
    margin: 0.85rem 0 0;
    color: #cbd5e1;
    line-height: 1.7;
  }

  .status-card {
    padding: 1.25rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 20px;
    background: rgba(15, 23, 42, 0.65);
  }

  .status-card strong {
    display: block;
    margin-bottom: 0.7rem;
  }

  .status-card ul {
    margin: 0;
    padding-left: 1.1rem;
    color: #cbd5e1;
    line-height: 1.7;
  }

  .summary {
    margin-top: 0.85rem;
    color: #94a3b8;
    line-height: 1.5;
  }

  .workspace {
    display: grid;
    grid-template-columns: minmax(280px, 320px) minmax(0, 1fr);
    gap: 1rem;
    align-items: start;
  }

  .sidebar,
  .content {
    display: grid;
    gap: 1rem;
  }

  @media (max-width: 980px) {
    .hero,
    .workspace {
      grid-template-columns: 1fr;
    }

    .page-shell {
      padding: 1rem;
    }
  }
</style>
