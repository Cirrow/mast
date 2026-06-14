<script lang="ts">
    import { onMount } from "svelte";

    let user = $state<any>(null);
    let loading = $state(true);

    onMount(async () => {
        const res = await fetch("/api/auth/me");
        if (res.status === 401) {
            window.location.href = "/signin";
            return;
        }
        user = await res.json();
        loading = false;
    });

    async function logout() {
        await fetch("/api/auth/logout", { method: "POST" });
        window.location.href = "/wiki/home";
    }
</script>

{#if loading}
    <p>Loading...</p>
{:else if user}
    <section class="account">
        <header>
            <h2>Hello, {user.login}</h2>
            <div class="profile-card">
                {#if user.avatar_url}
                    <img src={user.avatar_url} alt="avatar" class="avatar" />
                {/if}
                <div class="info">
                    <p>Authenticated via <strong>GitHub</strong></p>
                    <p class="email">{user.email || "No public email"}</p>
                </div>
            </div>
        </header>
        <button onclick={logout} class="btn-danger">Sign Out</button>
    </section>
{/if}

<style>
    .account { padding: 2rem; display: flex; flex-direction: column; gap: 2rem; }
    .profile-card { display: flex; align-items: center; gap: 1.5rem; background: rgba(255,255,255,0.05); border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); padding: 1.5rem; }
    .avatar { width: 80px; height: 80px; border-radius: 50%; border: 2px solid var(--accent-color, #ccc); object-fit: cover; }
    .email { opacity: 0.7; font-size: 0.9rem; }
    .btn-danger { align-self: flex-start; background: #24292f; color: white; padding: 12px 24px; border-radius: 6px; border: none; cursor: pointer; font-weight: 600; font-size: 1rem; }
    .btn-danger:hover { background: #000; }
</style>
