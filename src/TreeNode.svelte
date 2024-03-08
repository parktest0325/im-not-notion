<script lang="ts">
    import { writable } from "svelte/store";
    import TreeNode from "./TreeNode.svelte";
    import { selectedFilePath } from "./stores";

    export let path: string = "/";
    export let node: FileSystemNode;

    const isExpanded = writable(false);

    function toggleExpand() {
        if (node.type_ === "Directory") {
            isExpanded.update((value) => !value);
        }
        console.log($isExpanded);
    }

    function onFileClick(filePath: string) {
        // 나중에 클릭 핸들러 구현
        console.log(`File clicked: ${filePath}`);
        selectedFilePath.set(filePath);
    }
</script>

<li>
    <button
        class="flex items-center"
        on:click={() => {
            node.type_ === "File"
                ? onFileClick(`${path}${node.name}`)
                : toggleExpand();
        }}
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
                <TreeNode path={`${path}${node.name}/`} node={child} />
            {/each}
        </ul>
    {/if}
</li>

<style>
    li {
        list-style-type: none;
    }
</style>
