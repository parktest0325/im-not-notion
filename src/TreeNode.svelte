<script lang="ts">
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";

    export let node: FileSystemNode;

    const isExpanded = writable(false);

    function toggleExpand() {
        if (node.type_ === "Directory") {
            isExpanded.update((value) => !value);
        }
        console.log($isExpanded);
    }

    function onFileClick(name: string) {
        // 나중에 클릭 핸들러 구현
        console.log(`File clicked: ${name}`);
    }
</script>

<li>
    <button
        class="flex items-center"
        on:click={toggleExpand}
        aria-label="Toggle Expand"
    >
        {#if node.type_ === "Directory"}
            <span>{$isExpanded ? "▼" : "▶︎"}</span>
        {/if}
        <span class={node.type_ === "File" ? "cursor-pointer" : ""}>
            {node.name}
        </span>
    </button>
    {#if node.type_ === "Directory" && $isExpanded}
        <ul class="pl-4">
            {#each node.children as child}
                <TreeNode node={child} />
            {/each}
        </ul>
    {/if}
</li>

<style>
    li {
        list-style-type: none;
    }
</style>
