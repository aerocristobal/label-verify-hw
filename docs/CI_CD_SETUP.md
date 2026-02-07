# CI/CD Configuration for Cloudflare Credentials

This guide covers how to configure Cloudflare credentials in various CI/CD platforms for automated testing.

---

## Prerequisites

Before configuring CI/CD:

1. Create a dedicated R2 bucket for testing: `label-verify-test`
2. Generate R2 API token scoped to the test bucket
3. Use the same Workers AI token as development (or create a separate one)
4. Set up lifecycle policy to auto-delete test objects >7 days

See [CLOUDFLARE_SETUP.md](./CLOUDFLARE_SETUP.md) for detailed credential generation steps.

---

## GitHub Actions

### 1. Add Repository Secrets

Navigate to: **Repository → Settings → Secrets and variables → Actions → New repository secret**

Add these secrets:

| Secret Name | Value | Description |
|-------------|-------|-------------|
| `CF_ACCOUNT_ID` | `abc123...` | Cloudflare account ID |
| `CF_API_TOKEN` | `xxx...` | Workers AI API token |
| `R2_ACCESS_KEY_TEST` | `xxx...` | R2 access key for test bucket |
| `R2_SECRET_KEY_TEST` | `xxx...` | R2 secret key for test bucket |
| `R2_ENDPOINT` | `https://...` | R2 endpoint URL |
| `ENCRYPTION_KEY` | `base64...` | AES-256-GCM key for testing |
| `DATABASE_URL` | `postgresql://...` | Test database URL |
| `REDIS_URL` | `redis://...` | Test Redis URL |

### 2. Workflow Configuration

Create `.github/workflows/test.yml`:

```yaml
name: Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Run Tests
    runs-on: ubuntu-latest

    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: labelverify_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        env:
          # Cloudflare credentials
          CF_ACCOUNT_ID: ${{ secrets.CF_ACCOUNT_ID }}
          CF_API_TOKEN: ${{ secrets.CF_API_TOKEN }}
          R2_BUCKET: label-verify-test
          R2_ACCESS_KEY: ${{ secrets.R2_ACCESS_KEY_TEST }}
          R2_SECRET_KEY: ${{ secrets.R2_SECRET_KEY_TEST }}
          R2_ENDPOINT: ${{ secrets.R2_ENDPOINT }}

          # Other services
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/labelverify_test
          REDIS_URL: redis://localhost:6379
          ENCRYPTION_KEY: ${{ secrets.ENCRYPTION_KEY }}

          # Mock Azure AD for tests (or use real credentials)
          AZURE_TENANT_ID: test-tenant-id
          AZURE_CLIENT_ID: test-client-id

          BIND_ADDR: 0.0.0.0:3000
        run: cargo test --verbose

      - name: Run integration tests
        env:
          CF_ACCOUNT_ID: ${{ secrets.CF_ACCOUNT_ID }}
          CF_API_TOKEN: ${{ secrets.CF_API_TOKEN }}
          R2_BUCKET: label-verify-test
          R2_ACCESS_KEY: ${{ secrets.R2_ACCESS_KEY_TEST }}
          R2_SECRET_KEY: ${{ secrets.R2_SECRET_KEY_TEST }}
          R2_ENDPOINT: ${{ secrets.R2_ENDPOINT }}
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/labelverify_test
          REDIS_URL: redis://localhost:6379
          ENCRYPTION_KEY: ${{ secrets.ENCRYPTION_KEY }}
          AZURE_TENANT_ID: test-tenant-id
          AZURE_CLIENT_ID: test-client-id
          BIND_ADDR: 0.0.0.0:3000
        run: cargo test --test '*' -- --test-threads=1

  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - name: Run clippy
        run: cargo clippy -- -D warnings
      - name: Check formatting
        run: cargo fmt -- --check

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install cargo-audit
        run: cargo install cargo-audit
      - name: Run security audit
        run: cargo audit
```

### 3. Secret Scanning

Enable GitHub secret scanning:
1. Go to **Settings → Code security and analysis**
2. Enable **Secret scanning**
3. Enable **Push protection** (prevents accidental commits)

---

## GitLab CI

### 1. Add CI/CD Variables

Navigate to: **Settings → CI/CD → Variables → Expand → Add variable**

Add these variables (all **masked** and **protected**):

| Variable Name | Value | Flags |
|---------------|-------|-------|
| `CF_ACCOUNT_ID` | `abc123...` | Masked |
| `CF_API_TOKEN` | `xxx...` | Masked, Protected |
| `R2_ACCESS_KEY_TEST` | `xxx...` | Masked, Protected |
| `R2_SECRET_KEY_TEST` | `xxx...` | Masked, Protected |
| `R2_ENDPOINT` | `https://...` | Masked |
| `ENCRYPTION_KEY` | `base64...` | Masked, Protected |

### 2. Pipeline Configuration

Create `.gitlab-ci.yml`:

```yaml
image: rust:1.75

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo
  R2_BUCKET: label-verify-test
  POSTGRES_DB: labelverify_test
  POSTGRES_USER: postgres
  POSTGRES_PASSWORD: postgres

stages:
  - test
  - lint
  - security

cache:
  paths:
    - .cargo/
    - target/

services:
  - postgres:15
  - redis:7

before_script:
  - apt-get update -qq && apt-get install -y -qq postgresql-client redis-tools
  - rustc --version
  - cargo --version

test:
  stage: test
  variables:
    DATABASE_URL: postgresql://postgres:postgres@postgres:5432/labelverify_test
    REDIS_URL: redis://redis:6379
    AZURE_TENANT_ID: test-tenant-id
    AZURE_CLIENT_ID: test-client-id
    BIND_ADDR: 0.0.0.0:3000
  script:
    # Wait for services
    - until pg_isready -h postgres -p 5432 -U postgres; do sleep 1; done
    - until redis-cli -h redis ping; do sleep 1; done

    # Run tests
    - cargo test --verbose

    # Run integration tests
    - cargo test --test '*' -- --test-threads=1
  coverage: '/^\d+\.\d+% coverage/'

clippy:
  stage: lint
  script:
    - rustup component add clippy
    - cargo clippy -- -D warnings

rustfmt:
  stage: lint
  script:
    - rustup component add rustfmt
    - cargo fmt -- --check

audit:
  stage: security
  script:
    - cargo install cargo-audit
    - cargo audit
  allow_failure: true
```

---

## CircleCI

### 1. Add Environment Variables

Navigate to: **Project Settings → Environment Variables → Add Variable**

Add these variables:

- `CF_ACCOUNT_ID`
- `CF_API_TOKEN`
- `R2_ACCESS_KEY_TEST`
- `R2_SECRET_KEY_TEST`
- `R2_ENDPOINT`
- `ENCRYPTION_KEY`

### 2. Pipeline Configuration

Create `.circleci/config.yml`:

```yaml
version: 2.1

orbs:
  rust: circleci/rust@1.6

jobs:
  test:
    docker:
      - image: cimg/rust:1.75
      - image: postgres:15
        environment:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: labelverify_test
      - image: redis:7

    environment:
      DATABASE_URL: postgresql://postgres:postgres@localhost:5432/labelverify_test
      REDIS_URL: redis://localhost:6379
      R2_BUCKET: label-verify-test
      AZURE_TENANT_ID: test-tenant-id
      AZURE_CLIENT_ID: test-client-id
      BIND_ADDR: 0.0.0.0:3000

    steps:
      - checkout

      - restore_cache:
          keys:
            - cargo-cache-{{ arch }}-{{ checksum "Cargo.lock" }}

      - run:
          name: Wait for Postgres
          command: dockerize -wait tcp://localhost:5432 -timeout 1m

      - run:
          name: Wait for Redis
          command: dockerize -wait tcp://localhost:6379 -timeout 1m

      - run:
          name: Run tests
          command: cargo test --verbose

      - save_cache:
          paths:
            - ~/.cargo
            - target
          key: cargo-cache-{{ arch }}-{{ checksum "Cargo.lock" }}

  lint:
    docker:
      - image: cimg/rust:1.75
    steps:
      - checkout
      - run:
          name: Install tools
          command: rustup component add clippy rustfmt
      - run:
          name: Clippy
          command: cargo clippy -- -D warnings
      - run:
          name: Format check
          command: cargo fmt -- --check

workflows:
  version: 2
  test-and-lint:
    jobs:
      - test
      - lint
```

---

## Jenkins

### 1. Add Credentials

Navigate to: **Manage Jenkins → Manage Credentials → Add Credentials**

Add these as **Secret text**:

- `cf-account-id`
- `cf-api-token`
- `r2-access-key-test`
- `r2-secret-key-test`
- `r2-endpoint`
- `encryption-key`

### 2. Pipeline Configuration

Create `Jenkinsfile`:

```groovy
pipeline {
    agent {
        docker {
            image 'rust:1.75'
        }
    }

    environment {
        CARGO_HOME = "${WORKSPACE}/.cargo"
        R2_BUCKET = 'label-verify-test'
        DATABASE_URL = 'postgresql://postgres:postgres@postgres:5432/labelverify_test'
        REDIS_URL = 'redis://redis:6379'
        AZURE_TENANT_ID = 'test-tenant-id'
        AZURE_CLIENT_ID = 'test-client-id'
        BIND_ADDR = '0.0.0.0:3000'
    }

    stages {
        stage('Setup') {
            steps {
                sh 'rustc --version'
                sh 'cargo --version'
            }
        }

        stage('Test') {
            environment {
                CF_ACCOUNT_ID = credentials('cf-account-id')
                CF_API_TOKEN = credentials('cf-api-token')
                R2_ACCESS_KEY = credentials('r2-access-key-test')
                R2_SECRET_KEY = credentials('r2-secret-key-test')
                R2_ENDPOINT = credentials('r2-endpoint')
                ENCRYPTION_KEY = credentials('encryption-key')
            }
            steps {
                sh 'cargo test --verbose'
            }
        }

        stage('Lint') {
            steps {
                sh 'rustup component add clippy rustfmt'
                sh 'cargo clippy -- -D warnings'
                sh 'cargo fmt -- --check'
            }
        }

        stage('Security Audit') {
            steps {
                sh 'cargo install cargo-audit'
                sh 'cargo audit'
            }
        }
    }

    post {
        always {
            cleanWs()
        }
    }
}
```

---

## Best Practices

### Security

1. **Never print secrets in logs**:
   ```yaml
   # ❌ BAD
   - run: echo "Token is $CF_API_TOKEN"

   # ✅ GOOD
   - run: echo "Token configured"
   ```

2. **Use secret scanning**:
   - Enable platform-native secret scanning
   - Use tools like GitGuardian, TruffleHog

3. **Rotate credentials quarterly**:
   - Set calendar reminders
   - Document rotation procedures
   - Test after rotation

4. **Scope permissions**:
   - Use minimum required permissions
   - Separate tokens per environment

### Data Management

1. **Clean up test data**:
   ```yaml
   # Add cleanup job
   cleanup:
     stage: cleanup
     when: always
     script:
       - cargo run --bin cleanup-test-data
   ```

2. **Set R2 lifecycle policies**:
   - Auto-delete objects >7 days in test bucket
   - Monitor storage usage

3. **Mock external services when possible**:
   ```rust
   #[cfg(test)]
   mod tests {
       use mockito;

       #[tokio::test]
       async fn test_with_mock() {
           // Mock Workers AI instead of calling real API
           let mock = mockito::mock("POST", "/ai/run/...")
               .with_status(200)
               .with_body(r#"{"result":...}"#)
               .create();

           // Run test...
       }
   }
   ```

### Cost Control

1. **Limit test runs**:
   - Run full integration tests only on main branch
   - Use unit tests with mocks for PRs

2. **Monitor usage**:
   - Track Workers AI inference count
   - Monitor R2 storage growth
   - Set up billing alerts

3. **Optimize test suite**:
   - Cache test results
   - Parallelize where safe
   - Skip optional external calls

---

## Troubleshooting

### "Secret not found" Error

**GitHub Actions**:
- Verify secret name matches exactly (case-sensitive)
- Check secret is added at repository level (not organization)
- Ensure workflow has permission to access secrets

**GitLab CI**:
- Verify variable is not protected-only on non-protected branch
- Check variable is not environment-specific
- Ensure mask settings don't hide needed characters

### Tests Pass Locally But Fail in CI

1. **Check environment differences**:
   - Database version mismatch
   - Redis version mismatch
   - Missing environment variables

2. **Verify service readiness**:
   ```yaml
   # Add health checks
   - name: Wait for services
     run: |
       until pg_isready; do sleep 1; done
       until redis-cli ping; do sleep 1; done
   ```

3. **Enable debug logging**:
   ```yaml
   env:
     RUST_LOG: debug
     RUST_BACKTRACE: 1
   ```

### Rate Limiting

If you hit Cloudflare rate limits:

1. **Reduce test frequency**:
   - Run integration tests only on main branch
   - Use mocks for unit tests

2. **Add delays between requests**:
   ```rust
   tokio::time::sleep(Duration::from_millis(100)).await;
   ```

3. **Use separate account for testing**:
   - Isolate quota from development

---

## Next Steps

1. Set up credentials in your CI/CD platform
2. Test the pipeline with a simple commit
3. Monitor first few runs for issues
4. Document any platform-specific quirks

For more information, see [CLOUDFLARE_SETUP.md](./CLOUDFLARE_SETUP.md).
