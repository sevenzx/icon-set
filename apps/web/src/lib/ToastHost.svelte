<script lang="ts">
  import { removeToast, toasts } from '$lib/toast';
</script>

{#if $toasts.length > 0}
  <section class="toast-host" aria-live="polite" aria-label="页面提示">
    {#each $toasts as item (item.id)}
      <article class:error={item.variant === 'error'} class="toast-card">
        <div class="toast-copy">
          <span>{item.variant === 'error' ? 'Error' : 'Info'}</span>
          <p>{item.message}</p>
        </div>
        <button type="button" aria-label="关闭提示" on:click={() => removeToast(item.id)}>×</button>
      </article>
    {/each}
  </section>
{/if}

<style>
  .toast-host {
    position: fixed;
    top: 20px;
    left: 50%;
    z-index: 80;
    display: grid;
    gap: 10px;
    width: min(420px, calc(100vw - 32px));
    transform: translateX(-50%);
    pointer-events: none;
  }

  .toast-card {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
    padding: 14px 14px 14px 16px;
    border: 1px solid rgba(198, 255, 72, 0.44);
    border-radius: 18px;
    color: #f6efd9;
    background:
      linear-gradient(135deg, rgba(198, 255, 72, 0.14), transparent 42%),
      rgba(12, 13, 11, 0.92);
    box-shadow: 0 18px 60px rgba(0, 0, 0, 0.38), 4px 4px 0 rgba(255, 85, 36, 0.95);
    backdrop-filter: blur(18px);
    pointer-events: auto;
    animation: toast-in 180ms ease-out;
  }

  .toast-card.error {
    border-color: rgba(255, 85, 36, 0.58);
    background:
      linear-gradient(135deg, rgba(255, 85, 36, 0.2), transparent 46%),
      rgba(12, 13, 11, 0.94);
    box-shadow: 0 18px 60px rgba(0, 0, 0, 0.42), 4px 4px 0 rgba(198, 255, 72, 0.92);
  }

  .toast-copy {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .toast-copy span {
    color: #c6ff48;
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  .toast-card.error .toast-copy span {
    color: #ff7a52;
  }

  .toast-copy p {
    margin: 0;
    color: rgba(246, 239, 217, 0.9);
    font-size: 13px;
    line-height: 1.55;
    overflow-wrap: anywhere;
  }

  .toast-card button {
    display: grid;
    width: 28px;
    height: 28px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 999px;
    color: rgba(246, 239, 217, 0.82);
    background: rgba(246, 239, 217, 0.08);
    line-height: 1;
  }

  .toast-card button:hover {
    border-color: rgba(246, 239, 217, 0.36);
    background: rgba(246, 239, 217, 0.14);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (max-width: 640px) {
    .toast-host {
      top: 14px;
      width: calc(100vw - 28px);
    }
  }
</style>
