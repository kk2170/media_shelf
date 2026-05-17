<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  import type { Tag, TagCategory } from '$lib/types';

  const dispatch = createEventDispatcher<{
    toggle: { tagId: number };
    clear: void;
  }>();

  export let categories: TagCategory[] = [];
  export let tags: Tag[] = [];
  export let selectedTagIds: number[] = [];

  const tagsByCategory = (categoryId: number) => tags.filter((tag) => tag.categoryId === categoryId);
  const isSelected = (tagId: number) => selectedTagIds.includes(tagId);
</script>

<aside class="panel">
  <div class="panel-header">
    <div>
      <h2>タグフィルタ</h2>
      <p>複数タグをAND条件で組み合わせてライブラリを絞り込みます。</p>
    </div>

    <button class="clear-button" disabled={selectedTagIds.length === 0} on:click={() => dispatch('clear')}>
      クリア
    </button>
  </div>

  <div class="category-list">
    {#each categories as category}
      <section class="category-block">
        <header>
          <span class="dot" style={`background:${category.color ?? '#64748b'}`}></span>
          <strong>{category.name}</strong>
        </header>

        {#if tagsByCategory(category.id).length > 0}
          <div class="chips">
            {#each tagsByCategory(category.id) as tag}
              <button
                class:active={isSelected(tag.id)}
                class="chip"
                on:click={() => dispatch('toggle', { tagId: tag.id })}
              >
                {tag.name}
              </button>
            {/each}
          </div>
        {:else}
          <p class="muted">まだタグがありません。</p>
        {/if}
      </section>
    {/each}
  </div>
</aside>

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
    font-size: 1.05rem;
  }

  p {
    margin: 0.35rem 0 0;
    color: #94a3b8;
  }

  .clear-button {
    border: 1px solid rgba(148, 163, 184, 0.28);
    border-radius: 999px;
    padding: 0.55rem 0.85rem;
    background: rgba(15, 23, 42, 0.5);
    color: #cbd5e1;
    font: inherit;
    cursor: pointer;
  }

  .clear-button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .category-list {
    display: grid;
    gap: 0.75rem;
  }

  .category-block {
    padding: 0.9rem;
    border-radius: 16px;
    background: rgba(30, 41, 59, 0.72);
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

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }

  .chip {
    border: 1px solid rgba(148, 163, 184, 0.25);
    border-radius: 999px;
    padding: 0.4rem 0.7rem;
    background: rgba(15, 23, 42, 0.4);
    color: #cbd5e1;
    font: inherit;
    cursor: pointer;
  }

  .chip.active {
    border-color: rgba(56, 189, 248, 0.9);
    background: rgba(14, 165, 233, 0.2);
    color: #e0f2fe;
  }

  .muted {
    margin-top: 0.75rem;
    font-size: 0.9rem;
  }
</style>
