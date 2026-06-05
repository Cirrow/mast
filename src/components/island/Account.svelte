<script lang="ts">
    import { onMount } from "svelte";
    
    let user = $state<any>(null)
    let loading = $state(true)


    onMount(async () => {
        const res = await fetch("/api/auth/me")
    })

</script>