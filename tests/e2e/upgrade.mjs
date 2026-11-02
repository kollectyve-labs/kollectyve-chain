// Runtime-upgrade rehearsal: 103 → 104 on a dev chain, done exactly the way it will be done on
// the testnet from Polkadot-JS Apps — `sudo.sudoUncheckedWeight(system.setCode(wasm))` signed by
// the sudo key — then the new calls exercised against the upgraded runtime.
//
//   <old node> --dev --tmp          # a node still running spec 103
//   WASM=../../target/release/wbuild/kollectyve-runtime/kollectyve_runtime.compact.compressed.wasm \
//     node upgrade.mjs
//
// Exits non-zero if any assertion fails.

import { readFileSync } from 'node:fs';
import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { u8aToHex } from '@polkadot/util';
import { blake2AsHex, cryptoWaitReady } from '@polkadot/util-crypto';

const WS = process.env.WS || 'ws://127.0.0.1:9944';
const WASM = process.env.WASM;
if (!WASM) throw new Error('set WASM to the compact compressed runtime');

let pass = 0;
let fail = 0;
function record(name, ok, detail = '') {
  ok ? pass++ : fail++;
  console.log(`${ok ? '  \x1b[32mPASS\x1b[0m' : '  \x1b[31mFAIL\x1b[0m'}  ${name}${detail ? `\n        ${detail}` : ''}`);
}

function send(tx, signer) {
  return new Promise((resolve) => {
    let unsub;
    tx.signAndSend(signer, ({ status, events = [], dispatchError }) => {
      if (!status.isInBlock) return;
      unsub?.();
      let err = null;
      if (dispatchError) {
        err = dispatchError.isModule
          ? (({ section, name }) => `${section}.${name}`)(dispatchError.registry.findMetaError(dispatchError.asModule))
          : dispatchError.toString();
      }
      // A sudo call succeeds at the outer level regardless; surface the inner result.
      for (const { event } of events) {
        if (event.section === 'sudo' && event.method === 'Sudid' && event.data[0].isErr) {
          const e = event.data[0].asErr;
          err = e.isModule ? (({ section, name }) => `${section}.${name}`)(e.registry.findMetaError(e.asModule)) : e.toString();
        }
      }
      resolve({ ok: !err, err, events: events.map((r) => `${r.event.section}.${r.event.method}`) });
    })
      .then((u) => (unsub = u))
      .catch((e) => resolve({ ok: false, err: `pool: ${e.message}`, events: [] }));
  });
}

const connect = () => ApiPromise.create({ provider: new WsProvider(WS), noInitWarn: true });

await cryptoWaitReady();
const kr = new Keyring({ type: 'sr25519' });
const [alice, bob, dave, eve] = ['//Alice', '//Bob', '//Dave', '//Eve'].map((u) => kr.addFromUri(u));

// ── before ────────────────────────────────────────────────────────────────────
let api = await connect();
const before = (await api.rpc.state.getRuntimeVersion()).specVersion.toNumber();
console.log(`\n\x1b[1mbefore\x1b[0m  specVersion ${before}`);
record('starts without the credentials pallet', !api.tx.kollectyveCredentials);

// State created on the old runtime must survive the upgrade.
let r = await send(api.tx.kollectyveId.registerIdentity(bob.address, blake2AsHex('bob')), alice);
const bobId = (await api.query.kollectyveId.identityOf(bob.address)).unwrap().toNumber();
record('identity registered on the old runtime', r.ok, r.err ?? '');

// ── the upgrade, as Polkadot-JS Apps does it ────────────────────────────────
console.log('\n\x1b[1mupgrade\x1b[0m');
const code = u8aToHex(readFileSync(WASM));
console.log(`  wasm ${WASM} (${(code.length / 2 / 1024).toFixed(0)} KiB), blake2 ${blake2AsHex(readFileSync(WASM))}`);
r = await send(
  api.tx.sudo.sudoUncheckedWeight(api.tx.system.setCode(code), { refTime: 0, proofSize: 0 }),
  alice,
);
// Don't judge this step by its events: polkadot-js decodes the upgrade block's events with the
// *new* metadata (the block's post-state holds the new code) and fails. Apps can show the same
// decode error on that one block. `lastRuntimeUpgrade` below is the reliable check.
console.log(`  setCode extrinsic: ${r.ok ? 'included' : r.err} (event decoding on this block is unreliable)`);
await api.disconnect();

// The new code is used from the next block on.
for (let i = 0; i < 30; i++) {
  api = await connect();
  if ((await api.rpc.state.getRuntimeVersion()).specVersion.toNumber() !== before) break;
  await api.disconnect();
  await new Promise((res) => setTimeout(res, 2000));
}
const after = (await api.rpc.state.getRuntimeVersion()).specVersion.toNumber();
console.log(`\n\x1b[1mafter\x1b[0m  specVersion ${after}`);
record('specVersion is 104', after === 104, `got ${after}`);
// Nodes report the new version from the setCode block on, but the new runtime only runs (and
// records itself here) from the next block.
let lru = 0;
for (let i = 0; i < 15 && lru !== 104; i++) {
  lru = (await api.query.system.lastRuntimeUpgrade()).unwrap().specVersion.toNumber();
  if (lru !== 104) await new Promise((res) => setTimeout(res, 2000));
}
record('system.lastRuntimeUpgrade records 104 (new runtime has executed)', lru === 104, `got ${lru}`);
record('credentials pallet is present', !!api.tx.kollectyveCredentials);
record('old identity survived the upgrade',
  (await api.query.kollectyveId.identityOf(bob.address)).unwrapOr(null)?.toNumber() === bobId);

// ── credentials on the upgraded chain ──────────────────────────────────────
console.log('\n\x1b[1mpallet-kollectyve-credentials\x1b[0m');
for (const who of [dave, eve]) {
  r = await send(api.tx.balances.transferKeepAlive(who.address, 1000n * 10n ** 12n), alice);
  const free = (await api.query.system.account(who.address)).data.free.toBigInt();
  record(`funded ${who.meta.name ?? who.address.slice(0, 8)}`, r.ok && free > 0n, r.err ?? `free ${free}`);
}
await send(api.tx.kollectyveId.registerIdentity(alice.address, blake2AsHex('alice')), alice);
await send(api.tx.kollectyveId.registerIdentity(dave.address, blake2AsHex('dave')), alice);
const aliceId = (await api.query.kollectyveId.identityOf(alice.address)).unwrap().toNumber();
const daveId = (await api.query.kollectyveId.identityOf(dave.address)).unwrap().toNumber();

r = await send(api.tx.sudo.sudo(api.tx.kollectyveCredentials.setTrustAnchor(aliceId, true)), alice);
record('set_trust_anchor via sudo', r.ok, r.err ?? '');
r = await send(api.tx.kollectyveCredentials.setTrustAnchor(aliceId, true), alice);
record('set_trust_anchor without sudo is BadOrigin', (r.err ?? '').includes('BadOrigin'), r.err ?? 'success');

const schema = blake2AsHex('diploma-schema-v1');
r = await send(api.tx.kollectyveCredentials.registerSchema(schema), bob);
record('register_schema', r.ok, r.err ?? '');

const accreditation = blake2AsHex('accreditation');
r = await send(api.tx.kollectyveCredentials.issue(bobId, schema, accreditation, null, null), alice);
record('anchor issues an accreditation', r.ok, r.err ?? '');

const diploma = blake2AsHex('diploma');
r = await send(api.tx.kollectyveCredentials.issue(daveId, schema, diploma, accreditation, null), bob);
record('university issues a diploma under it', r.ok && r.events.includes('kollectyveCredentials.CredentialIssued'),
  r.err ?? '');
r = await send(api.tx.kollectyveCredentials.issue(daveId, schema, blake2AsHex('x'), accreditation, null), dave);
record("borrowing someone else's authorisation is refused",
  r.err === 'kollectyveCredentials.AuthorizationNotHeld', r.err ?? 'success');

r = await send(api.tx.kollectyveCredentials.revoke(diploma), dave);
record('subject cannot revoke', r.err === 'kollectyveCredentials.NotIssuer', r.err ?? 'success');
r = await send(api.tx.kollectyveCredentials.revoke(diploma), bob);
const rec = (await api.query.kollectyveCredentials.credentials(diploma)).unwrap();
record('issuer revokes; the record stays', r.ok && rec.revokedAt.isSome, r.err ?? '');

// ── pre-rotation, with the commitment computed off-chain ───────────────────
console.log('\n\x1b[1mpre-rotation\x1b[0m');
// blake2_256(SCALE(AccountId32)) = blake2_256(public key): what the SDK and the wallet compute.
const commitment = blake2AsHex(eve.publicKey);
r = await send(api.tx.kollectyveId.commitNextController(daveId, commitment), dave);
record('commit_next_controller', r.ok, r.err ?? '');
r = await send(api.tx.kollectyveId.rotateController(daveId, bob.address), dave);
record('plain rotate is refused while pre-rotated', r.err === 'kollectyveId.PreRotationRequired', r.err ?? 'success');
r = await send(api.tx.kollectyveId.rotateToCommitted(daveId, null), eve);
const ctrl = (await api.query.kollectyveId.controllerOf(daveId)).unwrap().toString();
record('the committed key takes control (off-chain commitment matches the pallet)',
  r.ok && ctrl === eve.address, r.err ?? `controller ${ctrl}`);

await api.disconnect();
console.log(`\n${pass} passed, ${fail} failed`);
process.exit(fail ? 1 : 0);
