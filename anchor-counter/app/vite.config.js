import { sveltekit } from '@sveltejs/kit/vite';

const config = {
	plugins: [sveltekit()],
	define: {
		// This makes @project-serum/anchor's process error
		// not happen since it replaces all instances of
		// process.env.BROWSER with true.
		'process.env.BROWSER': true,
		'process.env.NODE_DEBUG': JSON.stringify('')
	}
};

export default config;
