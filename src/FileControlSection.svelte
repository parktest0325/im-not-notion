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

<div class="file-list-container">
    <div class="actions">
        <input
            type="text"
            placeholder="Search..."
            bind:value={searchTerm}
            on:input={() => searchFiles(searchTerm)}
        />
        <button>
            <FaSearch />
        </button>
        <button on:click={refreshList}>
            <IoMdRefresh />
        </button>
    </div>
    <ul>
        {#each files as file}
            <li>
                <span class="icon">{file.type === "folder" ? "📁" : "📄"}</span>
                {file.name}
            </li>
        {/each}
    </ul>
</div>

<style>
    .file-list-container {
        /* 스타일 정의 */
    }
    .actions {
        /* 액션 바 스타일 */
    }
    .icon {
        width: 16px; /* 아이콘 크기 조정 */
        height: 16px;
        margin-right: 8px;
    }
    input {
        /* 입력 필드 스타일 */
    }
    ul {
        list-style: none;
        padding: 0;
    }
    li {
        display: flex;
        align-items: center;
        margin-bottom: 5px;
    }
    .icon:before {
        content: attr(data-icon);
        /* 아이콘에 따른 스타일 */
    }
</style>
