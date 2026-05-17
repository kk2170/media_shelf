<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import type { Tag, TagCategory } from '$lib/types';

  const dispatch = createEventDispatcher<{
    createCategory: { name: string; color: string | null };
    createTag: { categoryId: number; name: string; description: string | null; color: string | null };
    updateMediaTags: { tagIds: number[] };
  }>();

  export let categories: TagCategory[] = [];
  export let tags: Tag[] = [];
  export let selectedMediaId: number | null = null;
  export let selectedTagIds: number[] = [];
  export let busy = false;

  let categoryName = '';
  let categoryColor = '#38bdf8';
  let tagCategoryId = 0;
  let tagName = '';
  let tagDescription = '';
  let tagColor = '#8b5cf6';

  $: if (categories.length > 0 && !categories.some((category) => category.id === tagCategoryId)) {
    tagCategoryId = categories[0].id;
  }

  const tagsByCategory = (categoryId: number) => tags.filter((tag) => tag.categoryId === categoryId);

  function submitCategory() {
    dispatch('createCategory', {
      name: categoryName,
      color: categoryColor || null
    });

    categoryName = '';
  }

  function submitTag() {
    dispatch('createTag', {
      categoryId: tagCategoryId,
      name: tagName,
      description: tagDescription || null,
      color: tagColor || null
    });

    tagName = '';
    tagDescription = '';
  }

  function toggleTag(tagId: number, checked: boolean) {
    const nextIds = checked
      ? [...selectedTagIds, tagId]
      : selectedTagIds.filter((currentTagId) => currentTagId !== tagId);

    dispatch('updateMediaTags', { tagIds: nextIds });
  }
</script>

<section class="panel">
  <div>
    <h2>タグ編集</h2>
    <p>カテゴリ作成・タグ作成・選択中メディアへのタグ付けを行います。</p>
  </div>

  <form class="form-grid" on:submit|preventDefault={submitCategory}>
    <strong>カテゴリを追加</strong>

    <label>
      <span>カテゴリ名</span>
      <input bind:value={categoryName} placeholder="例: イベント" />
    </label>

    <label>
      <span>色</span>
      <input bind:value={categoryColor} type="color" />
    </label>

    <button disabled={busy || categoryName.trim().length === 0} type="submit">カテゴリを保存</button>
  </form>

  <form class="form-grid" on:submit|preventDefault={submitTag}>
    <strong>タグを追加</strong>

    <label>
      <span>カテゴリ</span>
      <select bind:value={tagCategoryId} disabled={categories.length === 0}>
        {#each categories as category}
          <option value={category.id}>{category.name}</option>
        {/each}
      </select>
    </label>

    <label>
      <span>タグ名</span>
      <input bind:value={tagName} placeholder="例: シェンムー" />
    </label>

    <label>
      <span>説明（任意）</span>
      <input bind:value={tagDescription} placeholder="用途や補足メモ" />
    </label>

    <label>
      <span>色（任意）</span>
      <input bind:value={tagColor} type="color" />
    </label>

    <button disabled={busy || categories.length === 0 || tagName.trim().length === 0} type="submit">
      タグを保存
    </button>
  </form>

  <div class="assignment-block">
    <div>
      <strong>選択中メディアへのタグ付け</strong>
      <p>
        {#if selectedMediaId}
          チェックを切り替えると、選択中メディアへ即時反映されます。
        {:else}
          メディアを選択するとタグ付けできます。
        {/if}
      </p>
    </div>

    <div class="assignment-groups">
      {#each categories as category}
        <section class="assignment-category">
          <header>
            <span class="dot" style={`background:${category.color ?? '#64748b'}`}></span>
            <strong>{category.name}</strong>
          </header>

          {#if tagsByCategory(category.id).length > 0}
            <div class="checkbox-list">
              {#each tagsByCategory(category.id) as tag}
                <label class:disabled={!selectedMediaId || busy} class="checkbox-item">
                  <input
                    checked={selectedTagIds.includes(tag.id)}
                    disabled={!selectedMediaId || busy}
                    type="checkbox"
                    on:change={(event) => toggleTag(tag.id, (event.currentTarget as HTMLInputElement).checked)}
                  />
                  <span>{tag.name}</span>
                </label>
              {/each}
            </div>
          {:else}
            <p class="muted">このカテゴリにはまだタグがありません。</p>
          {/if}
        </section>
      {/each}
    </div>
  </div>
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

  .form-grid,
  .assignment-block {
    display: grid;
    gap: 0.85rem;
    padding: 1rem;
    border-radius: 16px;
    background: rgba(30, 41, 59, 0.72);
  }

  label {
    display: grid;
    gap: 0.45rem;
  }

  span {
    color: #cbd5e1;
    font-size: 0.92rem;
  }

  input,
  select {
    border: 1px solid rgba(148, 163, 184, 0.2);
    border-radius: 12px;
    padding: 0.8rem 0.9rem;
    background: rgba(15, 23, 42, 0.45);
    color: #cbd5e1;
    font: inherit;
  }

  input[type='color'] {
    padding: 0.2rem;
    min-height: 2.75rem;
  }

  button {
    justify-self: start;
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
    cursor: not-allowed;
  }

  .assignment-groups {
    display: grid;
    gap: 0.85rem;
  }

  .assignment-category {
    display: grid;
    gap: 0.75rem;
    padding: 0.85rem;
    border-radius: 14px;
    background: rgba(15, 23, 42, 0.42);
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }

  .dot {
    width: 0.75rem;
    height: 0.75rem;
    border-radius: 999px;
  }

  .checkbox-list {
    display: grid;
    gap: 0.5rem;
  }

  .checkbox-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: #e2e8f0;
  }

  .checkbox-item.disabled {
    opacity: 0.55;
  }

  .checkbox-item input {
    margin: 0;
    width: 1rem;
    height: 1rem;
    accent-color: #38bdf8;
  }

  .muted {
    font-size: 0.9rem;
  }
</style>
