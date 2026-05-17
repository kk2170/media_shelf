import { writable } from 'svelte/store';
import type { Tag, TagCategory } from '$lib/types';

export const tagCategories = writable<TagCategory[]>([]);
export const tags = writable<Tag[]>([]);

export function setTagData(categories: TagCategory[], nextTags: Tag[]) {
  tagCategories.set(categories);
  tags.set(nextTags);
}
