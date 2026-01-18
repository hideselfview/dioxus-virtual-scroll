import { test, expect, Page } from '@playwright/test';

async function countGridItems(page: Page): Promise<number> {
  return await page.locator('.virtual-grid-content > div').count();
}

async function scrollTo(page: Page, y: number) {
  await page.evaluate((scrollY) => window.scrollTo(0, scrollY), y);
  await page.waitForTimeout(200);
}

test.describe('VirtualGrid', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.virtual-grid-content', { timeout: 30000 });
    await page.waitForTimeout(500);
  });

  test('limits DOM elements via virtual scrolling', async ({ page }) => {
    // Set album count to 500
    await page.locator('input[type="number"]').fill('500');
    await page.waitForTimeout(300);

    const itemCount = await countGridItems(page);
    
    // With 500 albums, we should NOT have 500 items in DOM
    expect(itemCount).toBeLessThan(100);
    expect(itemCount).toBeGreaterThan(0);
    
    console.log(`DOM has ${itemCount} items (expected < 100 for 500 albums)`);
  });

  test('changes visible items when scrolling', async ({ page }) => {
    await page.locator('input[type="number"]').fill('500');
    await page.waitForTimeout(300);

    const initialIndices = await page.locator('.virtual-grid-content > div[data-index]').evaluateAll(
      els => els.map(el => parseInt(el.dataset.index!, 10))
    );
    
    await scrollTo(page, 3000);
    
    const scrolledIndices = await page.locator('.virtual-grid-content > div[data-index]').evaluateAll(
      els => els.map(el => parseInt(el.dataset.index!, 10))
    );
    
    expect(scrolledIndices[0]).toBeGreaterThan(initialIndices[0]);
    
    console.log('Initial first index:', initialIndices[0]);
    console.log('After scroll first index:', scrolledIndices[0]);
  });

  test('maintains reasonable DOM count while scrolling', async ({ page }) => {
    await page.locator('input[type="number"]').fill('500');
    await page.waitForTimeout(300);

    const counts: number[] = [];
    
    for (let y = 0; y <= 5000; y += 1000) {
      await scrollTo(page, y);
      const count = await countGridItems(page);
      counts.push(count);
    }
    
    const maxCount = Math.max(...counts);
    const minCount = Math.min(...counts);
    
    expect(maxCount - minCount).toBeLessThan(20);
    expect(maxCount).toBeLessThan(100);
    
    console.log('DOM counts at scroll positions:', counts);
  });

  test('resize updates layout correctly', async ({ page }) => {
    async function getColumnsInFirstRow(): Promise<number> {
      const items = page.locator('.virtual-grid-content > div[data-index]');
      const boxes = await items.evaluateAll(els => 
        els.slice(0, 10).map(el => ({ y: el.getBoundingClientRect().y }))
      );
      if (boxes.length === 0) return 0;
      const firstRowY = boxes[0].y;
      return boxes.filter(b => Math.abs(b.y - firstRowY) < 5).length;
    }

    await page.setViewportSize({ width: 1200, height: 800 });
    await page.waitForTimeout(300);
    
    const wideCols = await getColumnsInFirstRow();
    
    await page.setViewportSize({ width: 500, height: 800 });
    await page.waitForTimeout(300);
    
    const narrowCols = await getColumnsInFirstRow();
    
    expect(wideCols).toBeGreaterThan(narrowCols);
    
    console.log(`Wide: ${wideCols} cols | Narrow: ${narrowCols} cols`);
  });

  test('remount works via cycle button', async ({ page }) => {
    const initialCount = await countGridItems(page);
    expect(initialCount).toBeGreaterThan(0);

    await page.getByRole('button', { name: 'Remount' }).click();
    await page.waitForTimeout(300);

    const afterCount = await countGridItems(page);
    expect(afterCount).toBeGreaterThan(0);
  });

  test('uses stable keys for DOM elements', async ({ page }) => {
    const items = page.locator('.virtual-grid-content > div[data-key]');
    const count = await items.count();
    expect(count).toBeGreaterThan(0);

    const firstKey = await items.first().getAttribute('data-key');
    expect(firstKey).toBeTruthy();
    expect(firstKey).toMatch(/^\d+$/);
    
    console.log(`Found ${count} items with data-key, first key: ${firstKey}`);
  });
});
