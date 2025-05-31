// /e:/prj/ghui/app/src/lib/Flash.svelte.ts

/**
 * Svelte action to make an element flash with a Tailwind CSS animation when updated.
 * Usage: <div use:flash>...</div>
 */
export function flash(node: HTMLElement) {
    // Tailwind classes for a nice flash animation
    const flashClass = 'animate-[flash_0.5s_ease-in-out]';

    // Define the keyframes for the flash animation in a style tag if not already present
    if (!document.getElementById('flash-keyframes')) {
        const style = document.createElement('style');
        style.id = 'flash-keyframes';
        style.textContent = `
            @keyframes flash {
                0%, 100% { background-color: transparent; }
                50% { background-color: rgba(255,255,0,0.5); }
            }
        `;
        document.head.appendChild(style);
    }

    function triggerFlash() {
        node.classList.remove(flashClass);
        // Force reflow to restart the animation
        void node.offsetWidth;
        node.classList.add(flashClass);
    }

    // Initial flash on mount
    triggerFlash();

    return {
        update() {
            triggerFlash();
        },
        destroy() {
            node.classList.remove(flashClass);
        }
    };
}