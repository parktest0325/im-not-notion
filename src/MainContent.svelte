<script lang="ts">
    import { onMount } from "svelte";
    import { selectedFilePath } from "./stores.js";
    import { invoke } from "@tauri-apps/api";

    // 파일 내용을 저장할 변수
    let fileContent: string = "";

    // selectedFilePath가 변경될 때마다 실행될 반응성 구문
    $: if ($selectedFilePath) {
        getFileContent($selectedFilePath);
    }

    async function getFileContent(filePath: string) {
        try {
            const content: string = await invoke("get_file_content", {
                filePath,
            });
            fileContent = content;
        } catch (error) {
            console.error("Failed to get file content", error);
            fileContent = "파일을 불러오는데 실패했습니다.";
        }
    }
</script>

<div class="main-content">
    <pre>{fileContent}</pre>
</div>
