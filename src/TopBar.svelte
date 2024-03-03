<script lang="ts">
    export let isMenuOpen: boolean;
    export let toggleMenu: () => void;
    import MdArrowForward from "svelte-icons/md/MdArrowForward.svelte";
</script>

<div class="top-bar" style="background-color:var(--pastel-purple);">
    {#if !isMenuOpen}
        <button on:click={toggleMenu} class="menu-button">
            <div class="icon">
                <MdArrowForward />
            </div>
        </button>

        <img src="inn.svg" alt="IM NOT NOTION!" />
    {/if}
    <span>Current Path</span>
</div>

<style>
    :root {
        --pastel-purple: #dcb6d5;
    }
    .top-bar {
        padding: 0.5em; /* 내부 요소 간격 */
        align-items: center;
        display: flex;
    }
    .menu-button {
        border: none; /* 테두리 제거 */
        background: none; /* 배경색 제거 */
        cursor: pointer; /* 마우스 오버 시 포인터 변경 */
        padding: 0.5em; /* 클릭 영역 확장 */
        font-size: 1em; /* 폰트 사이즈 조정 */
    }
    .logo {
        height: auto; /* 로고 크기 자동 조절 */
        width: 3em; /* 로고 너비 설정 */
        margin-right: 0.5em; /* 로고와 텍스트 간격 */
    }

    .icon {
        width: 24px;
        height: 24px;
    }
</style>
