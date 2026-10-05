/** Actual paragraph dialog, edit keys, undo/redo and saved final coordinates (#7490). */
import {runTest, createNewDocument, screenshot} from './helpers.mjs';
import assert from 'node:assert/strict';
const TEXT='1) 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하 가나다라마바사아자차카타파하';
const pause=page=>page.evaluate(()=>new Promise(resolve=>setTimeout(resolve,250)));
async function keys(page, modifier, key){await page.keyboard.down(modifier);await page.keyboard.press(key);await page.keyboard.up(modifier);await pause(page);}
async function state(page,para=0){return page.evaluate(p=>{
 const w=window.__wasm,h=window.__inputHandler;
 const rows=JSON.parse(w.doc.getPageTextLayout(0)).runs.filter(r=>r.secIdx===0&&r.paraIdx===p&&r.parentParaIdx===undefined&&r.text.trim());
 const starts=[];for(const r of rows){if(!starts.some(s=>Math.abs(s.y-r.y)<.5))starts.push({x:r.x,y:r.y});}
 return {starts,props:h.getParaProperties(),cursor:h.getCursorPosition(),paragraphs:w.doc.getParagraphCount(0),undo:h.history.undoStack.length,title:document.title};
},para);}
async function format(page,kind,pt){
 await page.evaluate(()=>window.__inputHandler.textarea.focus());await keys(page,'Alt','KeyT');
 await page.waitForSelector('input[name="ps-first-line"]');
 await page.click(`input[name="ps-first-line"][value="${kind}"]`);
 await page.evaluate(value=>{const radio=document.querySelector('input[name="ps-first-line"][value="indent"]');const input=radio.parentElement.querySelector('input[type="number"]');input.value=String(value);input.dispatchEvent(new Event('input',{bubbles:true}));input.dispatchEvent(new Event('change',{bubbles:true}));},pt);
 await page.evaluate(()=>{const button=[...document.querySelectorAll('button.dialog-btn-primary')].filter(b=>b.offsetParent!==null).at(-1);if(!button)throw Error('paragraph dialog apply button missing');button.click();});await pause(page);
}
function near(a,b,message){assert.ok(Math.abs(a-b)<.6,`${message}: ${a} vs ${b}`);}
await runTest('edited indent final coordinates through Studio history',async({page})=>{
 await page.evaluate(()=>{document.querySelector('.skin-onboarding-card')?.dispatchEvent(new MouseEvent('mousedown',{bubbles:true}));const button=[...document.querySelectorAll('button.dialog-btn-primary')].find(b=>b.offsetParent!==null&&b.textContent?.trim()==='시작하기');button?.click();});
 await createNewDocument(page);await page.evaluate(()=>window.__inputHandler.textarea.focus());await page.keyboard.type(TEXT);await pause(page);
 const flat=await state(page);assert.ok(flat.starts.length>=2);
 await format(page,'indent',15);const first=await state(page);near(first.starts[0].x-flat.starts[0].x,20,'15pt first-line indent');near(first.starts[1].x,flat.starts[1].x,'following line unchanged');assert.ok(first.undo>flat.undo);
 await keys(page,'Control','KeyZ');const undo=await state(page);near(undo.starts[0].x,flat.starts[0].x,'format undo');
 await keys(page,'Control','KeyY');near((await state(page)).starts[0].x,first.starts[0].x,'format redo');
 await format(page,'hanging',15);const hanging=await state(page);near(hanging.starts[0].x,flat.starts[0].x,'hanging first line');near(hanging.starts[1].x-flat.starts[1].x,20,'hanging continuation');
 await page.keyboard.type('가');await pause(page);const edited=await state(page);near(edited.starts[1].x,hanging.starts[1].x,'typing preserves hanging indent');assert.equal(edited.cursor.charOffset,TEXT.length+1);
 await keys(page,'Control','KeyZ');near((await state(page)).starts[1].x,hanging.starts[1].x,'typing undo preserves indent');await keys(page,'Control','KeyY');near((await state(page)).starts[1].x,hanging.starts[1].x,'typing redo preserves indent');
 await screenshot(page,'issue-7490-hanging-after-edit');
 await keys(page,'Control','End');await page.keyboard.press('Enter');await page.keyboard.type('다음문단');await pause(page);await format(page,'indent',15);const beforeMerge=await state(page,1);assert.equal(beforeMerge.paragraphs,2);
 await page.keyboard.press('Home');await page.keyboard.press('Backspace');await pause(page);assert.equal((await state(page)).paragraphs,1);
 await keys(page,'Control','KeyZ');const restored=await state(page,1);assert.equal(restored.paragraphs,2);near(restored.starts[0].x,beforeMerge.starts[0].x,'real Backspace merge undo restores indent');
 await keys(page,'Control','KeyY');assert.equal((await state(page)).paragraphs,1);await keys(page,'Control','KeyZ');assert.equal((await state(page)).paragraphs,2);
 const roundtrip=await page.evaluate(()=>{const w=window.__wasm;const bytes=w.exportHwp();w.loadDocument(bytes,'edited-indent-saved.hwp');return JSON.parse(w.doc.getPageTextLayout(0));});assert.ok(roundtrip.runs.some(r=>r.paraIdx===1&&r.text.includes('다음')),'saved text remains');
 await screenshot(page,'issue-7490-merge-undo-saved');console.log('PASS: dialog first/hanging + edit + repeated undo/redo + merge undo + saved final coordinates');
});
