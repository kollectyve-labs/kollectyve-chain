// End-to-end extrinsic tests against a live Kollectyve dev chain.
//
// The pallets' own unit tests run against mock runtimes, so they cannot catch a
// mis-wired origin, a wrong pallet index, or a genesis preset that does not fund who
// it claims to. This suite exercises the real runtime over RPC and does.
//
//   ../../scripts/dev-chain.sh      # in another shell
//   npm install && npm test
//
// Exits non-zero if any assertion fails, so it is CI-usable.

import { ApiPromise, WsProvider } from '@polkadot/api';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady, blake2AsHex } from '@polkadot/util-crypto';

const WS = process.env.WS || 'ws://127.0.0.1:9944';
const INCLUSION_TIMEOUT_MS = 90_000;

let pass = 0;
let fail = 0;

function record(name, ok, detail = '') {
  ok ? pass++ : fail++;
  const tag = ok ? '  \x1b[32mPASS\x1b[0m' : '  \x1b[31mFAIL\x1b[0m';
  console.log(`${tag}  ${name}${detail ? `\n        ${detail}` : ''}`);
}

// Submit and wait for inclusion. Always resolves — { ok, err, events } — so negative
// cases can assert on `err` instead of unwinding the suite.
function send(tx, signer) {
  return new Promise((resolve, reject) => {
    let unsub;
    const timer = setTimeout(() => {
      unsub?.();
      reject(new Error(`timed out waiting for inclusion after ${INCLUSION_TIMEOUT_MS}ms`));
    }, INCLUSION_TIMEOUT_MS);

    tx.signAndSend(signer, ({ status, events = [], dispatchError }) => {
      if (!status.isInBlock && !status.isFinalized) return;
      clearTimeout(timer);
      unsub?.();

      let err = null;
      if (dispatchError) {
        if (dispatchError.isModule) {
          const meta = dispatchError.registry.findMetaError(dispatchError.asModule);
          err = `${meta.section}.${meta.name}`;
        } else {
          err = dispatchError.toString();
        }
      }
      resolve({
        ok: !err,
        err,
        events: events.map((r) => `${r.event.section}.${r.event.method}`),
      });
    })
      .then((u) => { unsub = u; })
      .catch((e) => {
        clearTimeout(timer);
        // Pool-level rejection (unfunded account, bad nonce, …) never reaches dispatch,
        // so surface it as an error rather than tearing the suite down.
        resolve({ ok: false, err: `pool: ${e.message}`, events: [] });
      });
  });
}

const has = (r, name) => r.events.includes(name);

await cryptoWaitReady();
const api = await ApiPromise.create({ provider: new WsProvider(WS) });
const kr = new Keyring({ type: 'sr25519' });
const alice = kr.addFromUri('//Alice');
const bob = kr.addFromUri('//Bob');
const charlie = kr.addFromUri('//Charlie');
const dave = kr.addFromUri('//Dave');

const [chain, ver] = await Promise.all([api.rpc.system.chain(), api.rpc.system.version()]);
console.log(`\nConnected: ${chain} / ${ver}`);
const expected = ['kollectyveId', 'kollectyvePolicy', 'kumulus', 'revive'];
const present = expected.filter((p) => api.tx[p]);
console.log(`Pallets  : ${present.join(', ')}\n`);
if (present.length !== expected.length) {
  console.error(`missing pallets: ${expected.filter((p) => !api.tx[p]).join(', ')}`);
  process.exit(1);
}

// Charlie is NOT endowed in the dev genesis (only Alice, Bob and the stashes are), so
// fund him first — otherwise his calls die in the pool on the fee check and we never
// reach the pallet-level BadOrigin / NoIdentity errors we actually want to assert.
if ((await api.query.system.account(charlie.address)).data.free.isZero()) {
  const f = await send(api.tx.balances.transferKeepAlive(charlie.address, 10n ** 18n), alice);
  console.log(`funded Charlie: ${f.ok ? 'ok' : f.err}\n`);
}

// ── pallet-kollectyve-id ─────────────────────────────────────────────────────
console.log('\x1b[1mpallet-kollectyve-id\x1b[0m');

const nextId = (await api.query.kollectyveId.nextIdentityId()).toNumber();
const commitBob = blake2AsHex(`bob-subject-${Date.now()}`);

let r = await send(api.tx.kollectyveId.registerIdentity(bob.address, commitBob), alice);
record('register_identity (enrollment origin) succeeds',
  r.ok && has(r, 'kollectyveId.IdentityRegistered'), r.err || `events: ${r.events.join(', ')}`);

const bobId = (await api.query.kollectyveId.identityOf(bob.address)).unwrapOr(null);
record('IdentityOf(Bob) == freshly allocated id',
  bobId !== null && bobId.toNumber() === nextId, `got ${bobId?.toString()}, expected ${nextId}`);

const tier0 = await api.query.kollectyveId.tierOf(nextId);
record('new identity defaults to tier None', tier0.toString() === 'None', `got ${tier0.toString()}`);

r = await send(api.tx.kollectyveId.registerIdentity(charlie.address, commitBob), alice);
record('duplicate commitment rejected (CommitmentAlreadyUsed)',
  r.err === 'kollectyveId.CommitmentAlreadyUsed', `got ${r.err ?? 'success'}`);

r = await send(api.tx.kollectyveId.registerIdentity(dave.address, blake2AsHex(`dup-${Date.now()}`)), bob);
record('unprivileged origin rejected (BadOrigin)',
  (r.err ?? '').includes('BadOrigin'), `got ${r.err ?? 'success'}`);

r = await send(api.tx.kollectyveId.setTier(nextId, 'Verified'), alice);
record('set_tier (compliance origin) succeeds',
  r.ok && has(r, 'kollectyveId.TierSet'), r.err || `events: ${r.events.join(', ')}`);

const tier1 = await api.query.kollectyveId.tierOf(nextId);
record('TierOf reflects Verified', tier1.toString() === 'Verified', `got ${tier1.toString()}`);

r = await send(api.tx.kollectyveId.setTier(999_999, 'Basic'), alice);
record('set_tier on unknown identity rejected (UnknownIdentity)',
  r.err === 'kollectyveId.UnknownIdentity', `got ${r.err ?? 'success'}`);

r = await send(api.tx.kollectyveId.rotateController(nextId, dave.address), alice);
record('rotate_controller by non-controller rejected (NotController)',
  r.err === 'kollectyveId.NotController', `got ${r.err ?? 'success'}`);

// ── pallet-kumulus (needs a live identity; run before rotating Bob away) ─────
console.log('\n\x1b[1mpallet-kumulus\x1b[0m');

const specs = { cpuCores: 8, memoryMb: 16384, storageGb: 512, region: 686 };
r = await send(api.tx.kumulus.registerProvider(specs), bob);
record('register_provider by identity controller succeeds',
  r.ok && has(r, 'kumulus.ProviderRegistered'), r.err || `events: ${r.events.join(', ')}`);

const prov = (await api.query.kumulus.providers(nextId)).unwrapOr(null);
record('Providers record stored, active, specs round-trip',
  prov !== null && prov.active.isTrue &&
    prov.specs.cpuCores.toNumber() === 8 && prov.specs.region.toNumber() === 686,
  prov ? JSON.stringify(prov.toJSON()) : 'absent');

r = await send(api.tx.kumulus.registerProvider({ ...specs, cpuCores: 16 }), bob);
record('re-register emits ProviderUpdated (not Registered)',
  r.ok && has(r, 'kumulus.ProviderUpdated') && !has(r, 'kumulus.ProviderRegistered'),
  r.err || `events: ${r.events.join(', ')}`);

r = await send(api.tx.kumulus.registerProvider(specs), charlie);
record('register_provider without identity rejected (NoIdentity)',
  r.err === 'kumulus.NoIdentity', `got ${r.err ?? 'success'}`);

// ── pallet-kollectyve-policy ────────────────────────────────────────────────
console.log('\n\x1b[1mpallet-kollectyve-policy\x1b[0m');

const claim = { tier: 'Verified', jurisdiction: 686, accredited: false, expiresAt: null };
r = await send(api.tx.kollectyvePolicy.setClaim(nextId, claim), alice);
record('set_claim (compliance origin) succeeds',
  r.ok && has(r, 'kollectyvePolicy.ClaimSet'), r.err || `events: ${r.events.join(', ')}`);

const stored = (await api.query.kollectyvePolicy.claims(nextId)).unwrapOr(null);
record('Claims round-trip (tier + jurisdiction)',
  stored !== null && stored.tier.toString() === 'Verified' && stored.jurisdiction.toNumber() === 686,
  stored ? JSON.stringify(stored.toJSON()) : 'absent');

r = await send(api.tx.kollectyvePolicy.setClaim(999_999, claim), alice);
record('set_claim on unknown identity rejected (UnknownIdentity)',
  r.err === 'kollectyvePolicy.UnknownIdentity', `got ${r.err ?? 'success'}`);

const ASSET = 42;
const policy = {
  minTier: 'Verified',
  requireAccredited: false,
  allowedJurisdictions: [686],
  blockedJurisdictions: [],
  frozen: false,
};
r = await send(api.tx.kollectyvePolicy.setAssetPolicy(ASSET, policy), alice);
record('set_asset_policy succeeds',
  r.ok && has(r, 'kollectyvePolicy.AssetPolicySet'), r.err || `events: ${r.events.join(', ')}`);

r = await send(api.tx.kollectyvePolicy.setFrozen(ASSET, true), alice);
const frozen = (await api.query.kollectyvePolicy.assetPolicies(ASSET)).unwrapOr(null);
record('set_frozen flips the stored policy flag',
  r.ok && has(r, 'kollectyvePolicy.AssetFrozen') && frozen?.frozen.isTrue,
  r.err || `frozen=${frozen?.frozen.toString()}`);

r = await send(api.tx.kollectyvePolicy.setFrozen(4242, true), alice);
record('set_frozen on unknown asset rejected (UnknownAssetPolicy)',
  r.err === 'kollectyvePolicy.UnknownAssetPolicy', `got ${r.err ?? 'success'}`);

r = await send(api.tx.kollectyvePolicy.revokeClaim(nextId), alice);
const gone = await api.query.kollectyvePolicy.claims(nextId);
record('revoke_claim clears the claim',
  r.ok && has(r, 'kollectyvePolicy.ClaimRevoked') && gone.isNone,
  r.err || `claim=${gone.toString()}`);

r = await send(api.tx.kollectyvePolicy.setClaim(nextId, claim), charlie);
record('policy call from unprivileged origin rejected (BadOrigin)',
  (r.err ?? '').includes('BadOrigin'), `got ${r.err ?? 'success'}`);

// ── rotation last: it moves the identity away from Bob ───────────────────────
console.log('\n\x1b[1mpallet-kollectyve-id (rotation)\x1b[0m');

r = await send(api.tx.kollectyveId.rotateController(nextId, dave.address), bob);
record('rotate_controller by controller succeeds',
  r.ok && has(r, 'kollectyveId.ControllerRotated'), r.err || `events: ${r.events.join(', ')}`);

const [newCtrl, oldGone] = await Promise.all([
  api.query.kollectyveId.controllerOf(nextId),
  api.query.kollectyveId.identityOf(bob.address),
]);
record('ControllerOf -> Dave and IdentityOf(Bob) cleared',
  newCtrl.unwrapOr({ toString: () => '' }).toString() === dave.address && oldGone.isNone,
  `controller=${newCtrl.toString()}, bob=${oldGone.toString()}`);

console.log(
  `\n${'─'.repeat(64)}\n` +
  `Extrinsic suite: \x1b[32m${pass} passed\x1b[0m, ` +
  `${fail ? `\x1b[31m${fail} failed\x1b[0m` : '0 failed'}\n`,
);
await api.disconnect();
process.exit(fail ? 1 : 0);
