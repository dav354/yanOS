<script>
    let {
        message = '',
        title = 'Unsaved changes',
        cancelLabel = 'Cancel',
        confirmLabel = 'Save & Continue',
        onConfirm = () => {},
        onCancel = () => {},
    } = $props();
</script>

<div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
    <div class="bg-bg-card border border-border-main rounded shadow-lg max-w-sm w-full mx-4 p-4">
        <h3 class="text-lg font-semibold text-text-main mb-2">{title}</h3>
        <p class="text-text-muted text-sm mb-4">{message}</p>
        <div class="flex justify-end gap-2">
            <button
                class="px-3 py-2 text-sm rounded border border-border-main text-text-main hover:bg-bg-main"
                onclick={onCancel}
                type="button"
            >
                {cancelLabel}
            </button>
            <button
                class="px-3 py-2 text-sm rounded bg-primary text-primary-fg hover:bg-primary-hover"
                onclick={onConfirm}
                type="button"
            >
                {confirmLabel}
            </button>
        </div>
    </div>
</div>
