<script lang="ts">
  export let open = false;
  export let eyebrow = 'Delete';
  export let title = '确认删除';
  export let target = '';
  export let description = '';
  export let impact: string[] = [];
  export let confirmLabel = '';
  export let confirmHint = '';
  export let actionLabel = '永久删除';
  export let submitting = false;
  export let onCancel: () => void = () => {};
  export let onConfirm: () => void | Promise<void> = () => {};

  let step = 1;
  let confirmValue = '';
  let lastOpen = false;

  $: if (open !== lastOpen) {
    if (open) {
      step = 1;
      confirmValue = '';
    }
    lastOpen = open;
  }

  $: canSubmit = confirmValue === confirmLabel && !submitting;

  function cancel() {
    if (submitting) return;
    onCancel();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === 'Escape') {
      cancel();
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      cancel();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
  <div class="modal-backdrop" role="presentation" on:click={handleBackdropClick}>
    <div
      class="delete-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="delete-modal-title"
    >
      <div class="modal-head">
        <span class="eyebrow danger-eyebrow">{eyebrow}</span>
        <button class="close-button" type="button" aria-label="关闭" disabled={submitting} on:click={cancel}>
          ×
        </button>
      </div>

      <h2 id="delete-modal-title">{title}</h2>
      <p>{description}</p>

      <div class="target-block">
        <span>目标</span>
        <strong>{target}</strong>
      </div>

      {#if impact.length > 0}
        <ul class="impact-list">
          {#each impact as item}
            <li>{item}</li>
          {/each}
        </ul>
      {/if}

      {#if step === 1}
        <div class="modal-actions">
          <button class="action secondary" type="button" disabled={submitting} on:click={cancel}>取消</button>
          <button class="action danger" type="button" on:click={() => (step = 2)}>继续删除</button>
        </div>
      {:else}
        <label class="field confirm-field">
          <span>{confirmHint || `输入 ${confirmLabel} 继续`}</span>
          <input
            class="input"
            bind:value={confirmValue}
            autocomplete="off"
            spellcheck="false"
            disabled={submitting}
          />
        </label>
        <div class="modal-actions">
          <button class="action secondary" type="button" disabled={submitting} on:click={() => (step = 1)}>
            上一步
          </button>
          <button class="action danger" type="button" disabled={!canSubmit} on:click={onConfirm}>
            {submitting ? '删除中...' : actionLabel}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.68);
    backdrop-filter: blur(16px);
  }

  .delete-modal {
    display: grid;
    gap: 18px;
    width: min(620px, 100%);
    max-height: min(760px, calc(100vh - 40px));
    overflow: auto;
    padding: clamp(20px, 4vw, 32px);
    border: 1px solid rgba(255, 85, 36, 0.42);
    border-radius: 26px;
    color: #f6efd9;
    background:
      linear-gradient(135deg, rgba(255, 85, 36, 0.14), rgba(12, 13, 11, 0.92) 38%),
      #10110e;
    box-shadow: 0 32px 90px rgba(0, 0, 0, 0.62);
  }

  .modal-head,
  .modal-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .danger-eyebrow {
    border-color: rgba(255, 85, 36, 0.46);
    color: #ffd8c9;
    background: rgba(255, 85, 36, 0.14);
  }

  .close-button {
    display: grid;
    width: 40px;
    height: 40px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 10px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.06);
    font-size: 24px;
    line-height: 1;
  }

  .close-button:hover {
    border-color: rgba(255, 85, 36, 0.5);
    background: rgba(255, 85, 36, 0.16);
  }

  h2 {
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 48px;
    line-height: 0.96;
    letter-spacing: 0;
  }

  p {
    margin: 0;
    color: rgba(246, 239, 217, 0.72);
    line-height: 1.8;
  }

  .target-block {
    display: grid;
    gap: 8px;
    padding: 14px;
    border: 1px solid rgba(246, 239, 217, 0.14);
    border-radius: 8px;
    background: rgba(246, 239, 217, 0.05);
  }

  .target-block span,
  .confirm-field span {
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  .target-block strong {
    overflow-wrap: anywhere;
    color: #ff5524;
    font-size: 18px;
  }

  .impact-list {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .impact-list li {
    padding: 10px 12px;
    border-left: 3px solid rgba(255, 85, 36, 0.72);
    color: rgba(246, 239, 217, 0.72);
    background: rgba(255, 85, 36, 0.08);
    line-height: 1.6;
  }

  .confirm-field {
    gap: 10px;
  }

  .modal-actions {
    justify-content: flex-end;
  }

  .action.danger:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }

  @media (max-width: 640px) {
    .modal-actions {
      align-items: stretch;
      flex-direction: column;
    }

    h2 {
      font-size: 36px;
    }
  }
</style>
