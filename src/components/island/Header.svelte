<script lang="ts">
import config from "#config"

</script>

<nav class="nav-bar h-16 w-full fixed left-6 text-sm z-40">
    <ul class="flex items-center gap-6 list-none">
        <li class="mr-3"><a href="/wiki/home"><h1>{config.basic.name}</h1></a></li>
        <li>
            <a href="/about" class="a-no-style">
                About
            </a>
        </li>
        <li>
            <a href="/donate" class="a-no-style">
                Donate
            </a>
        </li>
        <li>
            <a href="/signin" class="a-no-style">
                Sign in
            </a>
        </li>
    </ul>
</nav>
