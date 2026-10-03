// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { luna } from './src/languages/luna.ts';

export default defineConfig({
	integrations: [
		starlight({
			title: 'Luna',
			description: 'A modern systems programming language with ownership-based memory safety, predictable native performance, and explicit low-level control.',
			defaultLocale: 'root',
			locales: {
				root: {
					label: 'English',
					lang: 'en',
				},
				vi: {
					label: 'Tiếng Việt',
					lang: 'vi',
				},
			},
			logo: {
				dark: './src/assets/brand/luna-lockup-dark.png',
				light: './src/assets/brand/luna-lockup-light.png',
				replacesTitle: true,
			},
			head: [
				{
					tag: 'link',
					attrs: {
						rel: 'icon',
						href: '/favicon.svg',
						type: 'image/svg+xml',
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'icon',
						href: '/favicon-32x32.png',
						type: 'image/png',
						sizes: '32x32',
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'apple-touch-icon',
						href: '/apple-touch-icon.png',
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'preconnect',
						href: 'https://fonts.googleapis.com',
					},
				},
				{
					tag: 'link',
					attrs: {
						rel: 'preconnect',
						href: 'https://fonts.gstatic.com',
						crossorigin: '',
					},
				},
				{
					tag: 'meta',
					attrs: {
						name: 'theme-color',
						content: '#0c0e14',
					},
				},
				{
					tag: 'script',
					content: `
						(() => {
							const syncTheme = () => {
								const isLight = document.documentElement.dataset.theme === 'light';
								const color = isLight ? '#ffffff' : '#0c0e14';
								let meta = document.querySelector('meta[name="theme-color"]');
								if (meta) meta.setAttribute('content', color);
							};
							new MutationObserver(syncTheme).observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
							syncTheme();
						})();
					`,
				},
			],
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/17thedevv/luna.git' },
			],
			customCss: ['./src/styles/custom.css'],
			expressiveCode: {
				themes: ['github-dark', 'github-light'],
				shiki: {
					langs: [luna],
				},
			},
			sidebar: [
				{
					label: 'Getting Started',
					slug: 'getting-started',
					translations: { vi: 'Bắt đầu' },
				},
				{
					label: 'Language Tour',
					translations: { vi: 'Khám phá ngôn ngữ' },
					items: [
						{ label: 'Overview', slug: 'language-tour', translations: { vi: 'Tổng quan' } },
						{ label: 'Variables & Types', slug: 'language-tour/variables', translations: { vi: 'Biến & Kiểu dữ liệu' } },
						{ label: 'Ownership & Borrowing', slug: 'language-tour/ownership', translations: { vi: 'Sở hữu & Vay mượn' } },
						{ label: 'Structs & Enums', slug: 'language-tour/structs-enums', translations: { vi: 'Struct & Enum' } },
						{ label: 'Traits & Generics', slug: 'language-tour/traits', translations: { vi: 'Trait & Generic' } },
						{ label: 'Modules', slug: 'language-tour/modules', translations: { vi: 'Module' } },
					],
				},
				{
					label: 'Standard Library',
					translations: { vi: 'Thư viện chuẩn' },
					items: [
						{ label: 'Overview', slug: 'stdlib', translations: { vi: 'Tổng quan' } },
						{ label: 'Option & Result', slug: 'stdlib/option-result', translations: { vi: 'Option & Result' } },
						{ label: 'Vec & Collections', slug: 'stdlib/vec-collections', translations: { vi: 'Vec & Collections' } },
						{ label: 'I/O', slug: 'stdlib/io', translations: { vi: 'I/O' } },
					],
				},
				{
					label: 'Reference',
					translations: { vi: 'Tham khảo' },
					items: [
						{ label: 'Syntax', slug: 'reference/syntax', translations: { vi: 'Cú pháp' } },
					],
				},
				{
					label: 'Roadmap',
					slug: 'roadmap',
					translations: { vi: 'Lộ trình' },
				},
			],
		}),
	],
});
