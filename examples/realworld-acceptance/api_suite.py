"""External HTTP assertions shared by reference Node and the real Niva backend.
This driver supplies no server implementation and cannot fulfill Native calls.
"""
import http.cookiejar
import json
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path


def run(base, database, verify_account=None):
    if urllib.parse.urlsplit(base).hostname not in ('127.0.0.1', 'localhost'):
        raise ValueError('Acceptance driver only targets an isolated loopback backend')
    database = Path(database)
    seed = json.loads(database.read_text())
    user, receiver = seed['users'][:2]
    jar = http.cookiejar.CookieJar()
    client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    checks, state = [], {}

    def request(method, route, body=None, expected=200):
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(base + route, data=data, method=method,
                                     headers={'Content-Type': 'application/json'})
        try:
            response = client.open(req, timeout=15)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            raw = response.read()
            if response.status != expected:
                raise AssertionError(f'{method} {route}: {response.status}, expected {expected}; {raw[:300]!r}')
            try:
                return json.loads(raw)
            except (ValueError, UnicodeDecodeError):
                return raw.decode(errors='replace')

    def check(name, callback):
        try:
            callback()
            checks.append({'name': name, 'ok': True})
        except Exception as error:
            checks.append({'name': name, 'ok': False, 'error': str(error)})

    def expect(condition, message):
        if not condition:
            raise AssertionError(message)

    def login():
        result = request('POST', '/login', {'username': user['username'], 'password': 's3cret', 'remember': True})
        assert result['user']['id'] == user['id']
        assert len(jar) > 0, 'Login did not establish a session cookie'

    if verify_account:
        check('login after backend restart', login)
        check('persisted account survives backend restart', lambda: expect(
            request('GET', '/bankAccounts/' + verify_account)['account']['id'] == verify_account,
            'Persisted account missing'))
        return {'ok': all(c['ok'] for c in checks), 'checks': checks, 'created': {'account': verify_account}}

    check('unauthorized request is rejected', lambda: request('GET', '/bankAccounts', expected=401))
    check('incorrect password is rejected', lambda: request('POST', '/login', {'username': user['username'], 'password': 'incorrect'}, expected=401))
    def register():
        username = 'niva-' + uuid.uuid4().hex[:12]
        value = request('POST', '/users', {
            'username': username, 'password': 'acceptance-password',
            'firstName': 'Niva', 'lastName': 'Acceptance',
            'email': username + '@example.test', 'balance': '10000',
            'defaultPrivacyLevel': 'public',
        }, expected=201)['user']
        assert value['username'] == username and value['id']
        state['registeredUser'] = value['id']
    check('register a user', register)
    check('login establishes session', login)
    check('session is recognized', lambda: expect(
        request('GET', '/checkAuth')['user']['id'] == user['id'], 'Session user mismatch'))
    check('authenticated user listing', lambda: expect(
        len(request('GET', '/users')['results']) > 0, 'Users missing'))
    check('invalid account data is rejected', lambda: request('POST', '/bankAccounts', {}, expected=422))

    def account():
        value = request('POST', '/bankAccounts', {'bankName': 'Niva Acceptance Bank', 'accountNumber': '123456789', 'routingNumber': '123456789'})['account']
        assert value['userId'] == user['id']
        state['account'] = value['id']
    check('create account', account)
    check('read newly created account', lambda: expect(
        request('GET', '/bankAccounts/' + state['account'])['account']['bankName'] == 'Niva Acceptance Bank',
        'Account content mismatch'))

    def contact():
        value = request('POST', '/contacts', {'contactUserId': receiver['id']})['contact']
        assert value['contactUserId'] == receiver['id']
        state['contact'] = value['id']
    check('create contact', contact)

    def payment():
        value = request('POST', '/transactions', {'transactionType': 'payment', 'receiverId': receiver['id'], 'description': 'Niva acceptance payment', 'amount': '100', 'privacyLevel': 'public'})['transaction']
        assert value['receiverId'] == receiver['id'] and value['senderId'] == user['id']
        state['transaction'] = value['id']
    check('create simulated payment', payment)
    check('read simulated payment', lambda: expect(
        request('GET', '/transactions/' + state['transaction'])['transaction']['description'] == 'Niva acceptance payment',
        'Transaction content mismatch'))
    check('notifications endpoint', lambda: expect(
        isinstance(request('GET', '/notifications')['results'], list), 'Notifications are not an array'))

    def persisted():
        data = json.loads(database.read_text())
        assert any(a['id'] == state['account'] for a in data['bankaccounts'])
        assert any(t['id'] == state['transaction'] for t in data['transactions'])
    check('account and transaction persisted to disk', persisted)
    check('logout invalidates session', lambda: request('POST', '/logout'))
    check('logged out session is rejected', lambda: request('GET', '/checkAuth', expected=401))
    return {'ok': all(c['ok'] for c in checks), 'checks': checks, 'created': state}
