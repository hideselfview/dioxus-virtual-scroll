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
    await page.locator('input[type="number"]').fill('500');
    await page.waitForTimeout(300);

    const itemCount = await countGridItems(page);
    
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

  test('items at correct absolute positions throughout scroll', async ({ page }) => {
    test.setTimeout(60000);
    await page.locator('input[type="number"]').fill('2000');
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.waitForTimeout(1000);

    const items = page.locator('.virtual-grid-content > div[data-index]');

    // Measure layout constants at scroll=0
    const layoutInfo = await page.evaluate(() => {
      const container = document.querySelector('.virtual-grid-container')!;
      const gridItems = document.querySelectorAll('.virtual-grid-content > div[data-index]');
      
      const containerRect = container.getBoundingClientRect();
      const containerPageOffset = containerRect.top + window.scrollY;
      
      const itemRects = Array.from(gridItems).slice(0, 20).map(el => el.getBoundingClientRect());
      const uniqueYs = [...new Set(itemRects.map(r => Math.round(r.y)))].sort((a, b) => a - b);
      const rowHeight = uniqueYs.length >= 2 ? uniqueYs[1] - uniqueYs[0] : 280;
      
      const firstRowY = itemRects[0]?.y ?? 0;
      const columns = itemRects.filter(r => Math.abs(r.y - firstRowY) < 5).length;
      
      return { containerPageOffset, rowHeight, columns };
    });

    console.log(`Layout: containerOffset=${layoutInfo.containerPageOffset}, rowHeight=${layoutInfo.rowHeight}, columns=${layoutInfo.columns}`);

    let previousPositions: Map<number, number> = new Map();
    let previousScrollY = 0;

    // Scroll in 20px increments
    for (let scrollY = 0; scrollY <= 5000; scrollY += 20) {
      await page.evaluate(y => window.scrollTo(0, y), scrollY);
      await page.waitForTimeout(50);

      const currentData = await items.evaluateAll(els => 
        els.map(el => ({
          index: parseInt(el.dataset.index!, 10),
          y: el.getBoundingClientRect().y
        }))
      );

      const currentPositions = new Map(currentData.map(d => [d.index, d.y]));
      const scrollDelta = scrollY - previousScrollY;
      const indices = [...currentPositions.keys()].sort((a, b) => a - b);
      
      if (scrollY % 500 === 0) {
        console.log(`scroll=${scrollY}: items ${indices[0]}-${indices[indices.length - 1]}, count=${indices.length}`);
      }

      // Items that existed before should have moved by exactly -scrollDelta
      if (previousPositions.size > 0) {
        for (const [index, prevY] of previousPositions) {
          if (currentPositions.has(index)) {
            const currentY = currentPositions.get(index)!;
            const actualDelta = currentY - prevY;
            const expectedDelta = -scrollDelta;
            
            expect(
              Math.abs(actualDelta - expectedDelta),
              `Item ${index} at scroll=${scrollY}: moved ${actualDelta}px, expected ${expectedDelta}px`
            ).toBeLessThan(2);
          }
        }
      }

      // Every item's absolute position should match expected based on its row
      for (const { index, y: actualY } of currentData) {
        const row = Math.floor(index / layoutInfo.columns);
        const expectedY = layoutInfo.containerPageOffset + (row * layoutInfo.rowHeight) - scrollY;
        const error = Math.abs(actualY - expectedY);
        
        expect(
          error,
          `Item ${index} absolute position at scroll=${scrollY}: Y=${actualY.toFixed(0)}, expected=${expectedY.toFixed(0)}`
        ).toBeLessThan(8);
      }

      previousPositions = currentPositions;
      previousScrollY = scrollY;
    }
  });

  test('scroll performance - bounded DOM churn', async ({ page }) => {
    await page.locator('input[type="number"]').fill('500');
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.waitForTimeout(500);

    await page.evaluate(() => {
      let count = 0;
      const observer = new MutationObserver(mutations => {
        count += mutations.length;
      });
      const target = document.querySelector('.virtual-grid-content');
      if (target) {
        observer.observe(target, { childList: true, subtree: true, attributes: true });
      }
      (window as any).__scrollMutationCount = () => {
        observer.disconnect();
        return count;
      };
    });

    for (let y = 0; y <= 8000; y += 200) {
      await page.evaluate(scrollY => window.scrollTo(0, scrollY), y);
      await page.waitForTimeout(30);
    }

    const mutationCount = await page.evaluate(() => (window as any).__scrollMutationCount());
    const itemCount = await countGridItems(page);
    
    console.log(`Scroll sweep: ${mutationCount} DOM mutations, ${itemCount} items in DOM`);
    
    expect(mutationCount, 'Too many DOM mutations during scroll').toBeLessThan(2000);
    expect(itemCount, 'Should still be virtualized after scroll').toBeLessThan(100);
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

  test('resize performance - debouncing works', async ({ page }) => {
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.waitForTimeout(500);

    await page.evaluate(() => {
      let count = 0;
      const observer = new MutationObserver(mutations => {
        count += mutations.length;
      });
      const target = document.querySelector('.virtual-grid-content');
      if (target) {
        observer.observe(target, { childList: true, subtree: true, attributes: true });
      }
      (window as any).__mutationCount = () => {
        observer.disconnect();
        return count;
      };
    });

    // Rapid resize (60 steps)
    for (let width = 1400; width >= 800; width -= 20) {
      await page.setViewportSize({ width, height: 900 });
      await page.waitForTimeout(16);
    }
    for (let width = 800; width <= 1400; width += 20) {
      await page.setViewportSize({ width, height: 900 });
      await page.waitForTimeout(16);
    }

    const finalCount = await page.evaluate(() => (window as any).__mutationCount());
    console.log(`Resize: 60 steps, ${finalCount} DOM mutations`);
    
    expect(finalCount, 'Too many DOM mutations during resize').toBeLessThan(200);
    
    const items = await page.locator('.virtual-grid-content > div[data-index]').count();
    expect(items).toBeGreaterThan(0);
    expect(items).toBeLessThan(100);
  });

  test('resize observer survives over time (no GC issues)', async ({ page }) => {
    await page.setViewportSize({ width: 400, height: 800 });
    await page.waitForTimeout(500);

    // Wait to give GC a chance
    await page.waitForTimeout(2000);
    
    await page.evaluate(() => {
      if ((window as any).gc) (window as any).gc();
    });
    
    await page.waitForTimeout(500);

    await page.setViewportSize({ width: 1200, height: 800 });
    await page.waitForTimeout(500);

    const items = page.locator('.virtual-grid-content > div[data-index]');
    const boxes = await items.evaluateAll(els => 
      els.slice(0, 10).map(el => ({ y: el.getBoundingClientRect().y }))
    );
    const firstRowY = boxes[0]?.y ?? 0;
    const cols = boxes.filter(b => Math.abs(b.y - firstRowY) < 5).length;
    
    console.log(`After GC wait + resize: ${cols} columns`);
    expect(cols, 'ResizeObserver should still work after potential GC').toBeGreaterThan(1);
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

  test('initial_scroll_to scrolls to specified item on remount', async ({ page }) => {
    await page.locator('input[type="number"]').fill('200');
    await page.waitForTimeout(300);

    // Should be at top initially
    let scrollY = await page.evaluate(() => window.scrollY);
    expect(scrollY).toBeLessThan(50);

    // Set scroll_to and remount
    await page.locator('input[type="text"]').fill('100');
    await page.getByRole('button', { name: 'Remount' }).click();
    await page.waitForTimeout(500);

    // Should have scrolled down
    scrollY = await page.evaluate(() => window.scrollY);
    expect(scrollY).toBeGreaterThan(500);

    // Item 100 should be visible
    const visibleKeys = await page.locator('.virtual-grid-content > div[data-key]').evaluateAll(
      els => els.map(el => parseInt(el.getAttribute('data-key')!, 10))
    );
    const minKey = Math.min(...visibleKeys);
    const maxKey = Math.max(...visibleKeys);
    
    // Item 100 should be in or near visible range
    expect(100).toBeGreaterThanOrEqual(minKey - 10);
    expect(100).toBeLessThanOrEqual(maxKey + 10);
    
    console.log(`scrollY: ${scrollY}, visible keys: ${minKey}-${maxKey}`);
  });

  test('cleanup - no memory leak on repeated mount/unmount', async ({ page }) => {
    test.setTimeout(180000);

    // Use CDP for accurate heap measurement
    const client = await page.context().newCDPSession(page);
    
    async function getHeapMB(): Promise<number> {
      await client.send('HeapProfiler.collectGarbage');
      const { usedSize } = await client.send('Runtime.getHeapUsage');
      return usedSize / 1024 / 1024;
    }

    await page.locator('input[type="number"]').fill('100');
    await page.waitForTimeout(500);

    const baseline = await getHeapMB();
    console.log(`Baseline heap: ${baseline.toFixed(2)} MB`);

    const CYCLES = 100;
    const measurements: { cycle: number; heap: number }[] = [];

    for (let i = 1; i <= CYCLES; i++) {
      await page.getByRole('button', { name: 'Remount' }).click();
      await page.waitForTimeout(30);

      if (i % 25 === 0) {
        const heap = await getHeapMB();
        measurements.push({ cycle: i, heap });
        console.log(`After ${i} cycles: ${heap.toFixed(2)} MB (Δ ${(heap - baseline).toFixed(2)} MB)`);
      }
    }

    const final = await getHeapMB();
    console.log(`\nFINAL: ${final.toFixed(2)} MB after ${CYCLES} cycles`);
    console.log(`Growth: ${(final - baseline).toFixed(2)} MB`);

    const growthPerCycle = (final - baseline) / CYCLES;
    console.log(`Growth per cycle: ${(growthPerCycle * 1024).toFixed(2)} KB`);

    // Without leaks, growth should be near 0 (GC cleans up)
    // Threshold of 5KB per cycle catches real leaks while allowing noise
    expect(growthPerCycle, 'Memory growing linearly - leak detected!').toBeLessThan(0.005);
  });
});
