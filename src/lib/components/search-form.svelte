<script lang="ts">
import SearchIcon from "lucide-svelte/icons/search";

</script>

<form>
    <div class="search-group">
        <div class="search-field">
            <label for="search" class="sr-only">Search</label>
            <input id="search" placeholder="Search the docs..." class="search-input" />
            <SearchIcon
                class="search-icon"
            />
        </div>
    </div>
</form>

<style>
    .search-group {
        padding: 0;
    }
    .search-field {
        position: relative;
    }
    .search-input {
        padding-left: 2rem;
        width: 100%;
    }
    .search-icon {
        pointer-events: none;
        position: absolute;
        left: 0.5rem;
        top: 50%;
        width: 1rem;
        height: 1rem;
        transform: translateY(-50%);
        opacity: 0.5;
        user-select: none;
    }
    .sr-only {
        position: absolute;
        width: 1px;
        height: 1px;
        padding: 0;
        margin: -1px;
        overflow: hidden;
        clip: rect(0, 0, 0, 0);
        border: 0;
    }
</style>
