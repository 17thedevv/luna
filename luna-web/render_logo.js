import sharp from 'sharp';
async function run() {
  await sharp('src/assets/logo-light.svg').png().toFile('test_logo_light.png');
  await sharp('src/assets/logo-dark.svg').png().toFile('test_logo_dark.png');
  console.log('Done rendering logos to PNG');
}
run();
