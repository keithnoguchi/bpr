<script>
	import { onMount } from 'svelte';

	const network = 'http://127.0.0.1:8899';

	onMount(async () => {
		console.log(`${network}`);
	});
</script>
<slot />
