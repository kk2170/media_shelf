export type MediaType = 'image' | 'video' | 'comic';

export interface MediaItem {
  id: number;
  fileName: string;
  filePath: string;
  sourcePath?: string | null;
  sourceUrl?: string | null;
  sourceTitle?: string | null;
  fileType: MediaType;
  mimeType?: string | null;
  fileSize?: number | null;
  thumbnailUrl?: string | null;
  importedAt: string;
  tagIds: number[];
  pageCount?: number | null;
  currentPageIndex?: number | null;
  seriesName?: string | null;
  completedAt?: string | null;
  lastViewedAt?: string | null;
}

export interface MediaPreviewData {
  assetUrl: string | null;
  thumbnailUrl: string | null;
  pageAssetUrls: string[] | null;
  pageCount: number | null;
  width: number | null;
  height: number | null;
  durationSeconds: number | null;
  status: 'loading' | 'ready' | 'error';
  errorMessage?: string | null;
}

export interface ImportMediaResult {
  folderPath: string;
  totalScanned: number;
  totalImported: number;
  mediaItems: MediaItem[];
}

export interface TagCategory {
  id: number;
  name: string;
  color: string | null;
  sortOrder: number;
}

export interface Tag {
  id: number;
  categoryId: number;
  name: string;
  description?: string | null;
  color?: string | null;
}

export interface LibrarySnapshot {
  mediaItems: MediaItem[];
  tagCategories: TagCategory[];
  tags: Tag[];
}

export interface ComicPagesResponse {
  mediaId: number;
  pagePaths: string[];
}

export interface SourceMetadataResponse {
  finalUrl: string;
  title: string;
  mediaItem: MediaItem;
}
