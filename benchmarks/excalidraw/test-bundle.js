const { chromium } = require('playwright');
const path = require('path');

async function testTalosBundle() {
  console.log('🧪 Testing Talos Bundle with Playwright\n');
  console.log('='.repeat(50));

  const browser = await chromium.launch({
    headless: true,  // Run in headless mode
  });

  const context = await browser.newContext();
  const page = await context.newPage();

  // Collect console messages
  const consoleMessages = [];
  page.on('console', msg => {
    consoleMessages.push(`[${msg.type()}] ${msg.text()}`);
    console.log(`  [Console ${msg.type()}] ${msg.text()}`);
  });

  // Collect errors
  const errors = [];
  page.on('pageerror', error => {
    errors.push(error.message);
    console.log(`  ❌ [Error] ${error.message}`);
  });

  try {
    console.log('\n📂 Loading test page...');
    const testPagePath = path.resolve(__dirname, 'excalidraw_subset/test-simple.html');
    await page.goto(`file://${testPagePath}`);

    // Wait for bundle to load
    console.log('⏳ Waiting for bundle to execute...');
    await page.waitForTimeout(3000);

    // Check if React root rendered
    console.log('\n🔍 Checking React rendering...');
    const rootElement = await page.$('#root');
    if (!rootElement) {
      console.log('  ❌ Root element not found');
      throw new Error('Root element not found');
    }
    console.log('  ✅ Root element found');

    // Check if React rendered content
    const rootContent = await page.$eval('#root', el => el.innerHTML);
    console.log(`\n📄 Root content length: ${rootContent.length} characters`);

    if (rootContent.length === 0) {
      console.log('  ❌ No content rendered');
      throw new Error('React did not render any content');
    }
    console.log('  ✅ Content rendered');

    // Check for specific React components
    const hasAppComponent = rootContent.includes('class') || rootContent.includes('div');
    console.log(`\n🔍 Checking for React components...`);
    if (!hasAppComponent) {
      console.log('  ⚠️  Warning: No obvious React component structure found');
    } else {
      console.log('  ✅ React components detected');
    }

    // Check for JSX transformation errors
    console.log('\n🔍 Checking for errors...');
    if (errors.length > 0) {
      console.log(`  ❌ Found ${errors.length} error(s):`);
      errors.forEach(err => console.log(`     - ${err}`));
    } else {
      console.log('  ✅ No errors detected');
    }

    // Take a screenshot
    const screenshotPath = path.resolve(__dirname, 'excalidraw_subset/screenshot-talos.png');
    await page.screenshot({ path: screenshotPath });
    console.log(`\n📸 Screenshot saved to: ${screenshotPath}`);

    // Final summary
    console.log('\n' + '='.repeat(50));
    console.log('📊 Test Summary:');
    console.log('='.repeat(50));
    console.log(`Root element: ${rootElement ? '✅ Found' : '❌ Missing'}`);
    console.log(`Content rendered: ${rootContent.length > 0 ? '✅ Yes' : '❌ No'}`);
    console.log(`Errors: ${errors.length === 0 ? '✅ None' : `❌ ${errors.length}`}`);
    console.log(`Console messages: ${consoleMessages.length}`);

    if (errors.length === 0 && rootContent.length > 0) {
      console.log('\n🎉 SUCCESS: Bundle executes correctly!');
      console.log('\n✅ Issue #101 is FIXED:');
      console.log('   - JSX transformation works');
      console.log('   - React renders successfully');
      console.log('   - No runtime errors');
    } else {
      console.log('\n❌ FAILURE: Bundle has issues');
      throw new Error('Test failed');
    }

  } catch (error) {
    console.error('\n❌ Test failed:', error.message);
    throw error;
  } finally {
    // Keep browser open for 2 seconds to see the result
    await page.waitForTimeout(2000);
    await browser.close();
  }
}

// Run the test
testTalosBundle()
  .then(() => {
    console.log('\n✅ Test completed successfully');
    process.exit(0);
  })
  .catch(error => {
    console.error('\n❌ Test failed:', error.message);
    process.exit(1);
  });
