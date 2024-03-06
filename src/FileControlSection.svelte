<script lang="ts">
    import FaSearch from "svelte-icons/fa/FaSearch.svelte";
    import IoMdRefresh from "svelte-icons/io/IoMdRefresh.svelte";
    let searchTerm: string = "";
    const refreshList = () => {
        // 리스트 새로고침 로직
    };

    const searchFiles = (term: string) => {
        // 검색 로직
    };

    const files = [
        { id: 1, name: "2024 계획", type: "folder" },
        { id: 2, name: "문서안안", type: "file" },
        // 나머지 파일 및 폴더 데이터...
    ];
</script>

<div class="flex space-x-2 h-6 mb-4" style="flex-wrap: nowrap;">
    <input
        type="text"
        placeholder="Search..."
        class="flex-grow p-2 border rounded"
        bind:value={searchTerm}
        on:input={() => searchFiles(searchTerm)}
        style="min-width: 0; width: auto; flex-grow: 1;"
    />
    <button>
        <div class="w-5 h-5">
            <FaSearch />
        </div>
    </button>
    <button on:click={refreshList}>
        <div class="w-5 h-5">
            <IoMdRefresh />
        </div>
    </button>
</div>
