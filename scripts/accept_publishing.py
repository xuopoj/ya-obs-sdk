#!/usr/bin/env python3
"""Plan locally; execute isolated publishing acceptance only in GitHub Actions."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from urllib.error import HTTPError
from urllib.request import urlopen
import uuid
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bucket', required=True)
    parser.add_argument('--region', default='cn-southwest-2')
    parser.add_argument('--cli', default='ya-obs')
    parser.add_argument('--execute', action='store_true')
    args = parser.parse_args()
    if not args.bucket or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-.' for c in args.bucket):
        parser.error('invalid bucket name')
    prefix = f'_ya-obs-acceptance/{uuid.uuid4().hex}/'
    keys = [prefix + n for n in ('public.zip', 'private.zip', 'race.zip')]
    print(json.dumps({'bucket': args.bucket, 'keys': keys, 'execute': args.execute}))
    if not args.execute:
        return
    if os.environ.get('GITHUB_ACTIONS') != 'true':
        parser.error('--execute is reserved for GitHub Actions')
    with tempfile.TemporaryDirectory() as tmp:
        base = [args.cli, '--config', str(Path(tmp)/'absent.toml'), '--region', args.region, '-q', '-o', 'json']
        def invoke(*cmd):
            return subprocess.run([*base, *map(str, cmd)], capture_output=True)
        def run(*cmd, expected=0):
            result = invoke(*cmd)
            if result.returncode != expected:
                raise RuntimeError(f'{cmd[0]} exit={result.returncode}; expected={expected}: {result.stderr.decode()}')
            return result
        def uri(key):
            return f'obs://{args.bucket}/{key}'
        def public_get(key):
            with urlopen(f'https://{args.bucket}.obs.{args.region}.myhuaweicloud.com/{key}', timeout=30) as response:
                return response.read()
        archive = Path(tmp)/'skill.zip'
        with zipfile.ZipFile(archive, 'w') as output:
            output.writestr('acceptance.txt', 'isolated test '+uuid.uuid4().hex)
        data = archive.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        options = ['--no-clobber', '--content-type', 'application/zip', '--cache-control', 'public, max-age=31536000, immutable',
                   '--content-disposition', 'attachment; filename="skill.zip"', '--metadata', f'sha256={digest}']
        created = []
        try:
            assert json.loads(run('bucket-versioning', f'obs://{args.bucket}').stdout)['versioning'] == 'Disabled'
            run('cp', archive, uri(keys[0]), *options, '--acl', 'public-read')
            created.append(keys[0])
            meta = json.loads(run('stat', uri(keys[0])).stdout)
            assert meta['size'] == len(data)
            assert meta['content_type'] == 'application/zip'
            assert meta['cache_control'] == options[4]
            assert meta['content_disposition'] == options[6]
            assert meta['metadata']['sha256'] == digest
            assert public_get(keys[0]) == data
            changed = Path(tmp)/'changed.zip'
            changed.write_bytes(b'different-content')
            conflict = run('cp', changed, uri(keys[0]), *options, expected=7)
            assert json.loads(conflict.stderr)['error']['status'] in (409, 412)
            assert public_get(keys[0]) == data
            run('cp', archive, uri(keys[1]), *options, '--acl', 'private')
            created.append(keys[1])
            try:
                public_get(keys[1])
            except HTTPError as err:
                assert err.code in (401, 403)
            else:
                raise AssertionError('test bucket allows anonymous GET despite private ACL')
            download = Path(tmp)/'download.zip'
            run('cp', uri(keys[1]), download)
            assert download.read_bytes() == data
            run('acl', 'set', uri(keys[1]), '--acl', 'public-read')
            assert public_get(keys[1]) == data
            def race(source):
                return invoke('cp', source, uri(keys[2]), *options, '--acl', 'public-read')
            with ThreadPoolExecutor(max_workers=2) as pool:
                results = list(pool.map(race, [archive, changed]))
            if any(r.returncode == 0 for r in results):
                created.append(keys[2])
            assert sorted(r.returncode for r in results) == [0, 7]
            loser = next(r for r in results if r.returncode == 7)
            assert json.loads(loser.stderr)['error']['status'] in (409, 412)
            winner = next(i for i, r in enumerate(results) if r.returncode == 0)
            assert public_get(keys[2]) == [data, changed.read_bytes()][winner]
            print(json.dumps({'ok': True, 'sha256': digest, 'bytes': len(data), 'prefix': prefix}))
        finally:
            errors = []
            for key in created:
                try:
                    run('rm', uri(key))
                except RuntimeError as err:
                    errors.append(str(err))
            if errors:
                raise RuntimeError('cleanup failed: '+'; '.join(errors))


if __name__ == '__main__':
    main()
