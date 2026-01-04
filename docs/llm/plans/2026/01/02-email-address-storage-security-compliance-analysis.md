# High-Level Analysis: Email Address Storage Security and Compliance

**Date**: 2026-01-02@19:37
**Status**: Analysis Phase
**Feature**: Multi-email address per user storage

## Executive Summary

This document analyzes security and compliance considerations for storing email addresses in an SQLite database as PII. The analysis focuses on practical, in-house solutions without external services like AWS, avoiding overly complex mechanisms like key rotation.

**Key Finding**: Deterministic field-level encryption using AES-256-GCM with proper key management provides the best balance of security, compliance, and usability for email address storage while maintaining search capabilities.

---

## Compliance Analysis

### Europe (GDPR)

**Requirement**: Article 32 mandates "appropriate technical and organizational measures" to ensure security. Encryption is not explicitly mandatory but is explicitly mentioned as a recommended safeguard in Recital 83 and recognized by supervisory authorities.

**Impact of Encryption**:
- Encryption reduces breach notification requirements (Article 33)
- Significantly reduces potential fines (up to €20 million or 4% of global turnover)
- Considered "appropriate security" by data protection authorities

**Encryption Standard**: N/A (no specific algorithm mandated, but "appropriate" is interpreted as strong encryption)

### United States

**Requirement**: No federal PII encryption law, but:
- **CCPA (California)**: Similar to GDPR - encryption is recommended and can provide safe harbor
- **HIPAA**: For healthcare data, encryption is required under the Security Rule
- **State breach notification laws**: Many states provide "safe harbor" if data is encrypted

**Impact of Encryption**:
- Reduces liability in breach scenarios
- Considered "reasonable security" by FTC
- State-specific safe harbor provisions

**Encryption Standard**: NIST-approved (e.g., AES-256)

### Canada (PIPEDA)

**Requirement**: Similar to GDPR - encryption is not explicitly mandated but is considered a "reasonable safeguard" under Principle 4.7 (Safeguards).

**Impact of Encryption**:
- Demonstrates due diligence
- Reduces breach impact
- Recommended by the Office of the Privacy Commissioner

**Encryption Standard**: N/A (no specific algorithm mandated)

### Brazil (LGPD)

**Requirement**: Article 46 requires "appropriate technical and organizational measures" to ensure security. Similar to GDPR structure.

**Impact of Encryption**:
- Encryption is recognized as an appropriate security measure
- Reduces administrative sanctions (up to 2% of revenue, capped at R$50 million)
- Required for "sensitive personal data" (which includes data that can identify individuals)

**Encryption Standard**: N/A (no specific algorithm mandated)

### Summary Table

| Jurisdiction | Encryption Required | Safe Harbor/Reduced Liability | Standard |
|--------------|---------------------|-------------------------------|----------|
| Europe (GDPR) | Recommended (not mandatory) | Yes | Strong encryption recommended |
| US | Varies by state | Yes (many states) | NIST-approved recommended |
| Canada (PIPEDA) | Recommended (not mandatory) | Yes | N/A |
| Brazil (LGPD) | Recommended (not mandatory) | Yes | Strong encryption recommended |

**Conclusion**: While no jurisdiction explicitly mandates encryption, all jurisdictions recognize encryption as an appropriate security measure and provide benefits in case of breaches. **AES-256-GCM meets all compliance requirements.**

---

## Storage Approach Analysis

### Option 1: Plain Text Storage

**Description**: Store email addresses as-is in the database.

**Pros**:
- Simple implementation
- Full search capabilities (exact match, pattern matching, substring search)
- No performance overhead

**Cons**:
- **Non-compliant** with PII protection requirements
- Data breach exposes email addresses directly
- No protection against unauthorized database access

**Verdict**: ❌ Not viable

---

### Option 2: Encoding (Base64)

**Description**: Encode email addresses using Base64 before storage.

**Pros**:
- Simple implementation
- Prevents casual inspection of database
- Easy to decode when needed

**Cons**:
- **Not encryption** - easily reversible
- **Non-compliant** - provides no real security
- No protection against determined attackers

**Verdict**: ❌ Not viable for PII

---

### Option 3: Hashing

**Description**: One-way hash of email addresses (e.g., SHA-256).

**Pros**:
- Cryptographically secure
- Cannot reverse to original email
- Good for authentication and uniqueness checks

**Cons**:
- **Cannot retrieve original email addresses**
- Only supports equality searches
- Susceptible to rainbow table attacks without salting

**Verdict**: ❌ Not viable if we need to retrieve/display email addresses

---

### Option 4: Full Database Encryption (SQLCipher)

**Description**: Encrypt entire SQLite database file using SQLCipher.

**Pros**:
- Transparent to application code
- All data encrypted (not just emails)
- Open source (but commercial license required for production)
- Strong AES-256 encryption

**Cons**:
- Requires commercial SQLCipher license for production
- Database must be fully decrypted to access any data
- External dependency on SQLCipher
- Requires integration with sqlx (additional complexity)
- **External dependency** (violates in-house preference)

**Verdict**: ❌ Not suitable due to external service requirement and licensing

---

### Option 5: Field-Level Encryption (Deterministic)

**Description**: Encrypt only email addresses using deterministic AES-256-GCM. Same plaintext always produces same ciphertext.

**Pros**:
- Strong encryption (AES-256-GCM)
- Supports equality searches (find if email exists)
- Application has full control
- No external dependencies
- In-house solution
- Minimal performance impact
- Can be implemented using existing `aes-gcm` crate

**Cons**:
- Pattern/substring searches not supported (e.g., "find all @gmail.com addresses")
- Deterministic encryption is theoretically vulnerable to frequency analysis attacks
- Requires secure key management

**Mitigations for Cons**:
- Pattern searches can be implemented by decrypting candidates in memory (not ideal but workable for small result sets)
- Add application-specific context (e.g., user ID) before encryption to reduce frequency analysis risk
- Use strong 256-bit keys to make frequency analysis impractical

**Verdict**: ✅ **Recommended** for email address storage

---

### Option 6: Field-Level Encryption (Randomized) with Blind Index

**Description**: Encrypt email addresses using randomized AES-256-GCM, store separate searchable hash index (blind index).

**Pros**:
- Strongest security (randomized encryption)
- Supports equality searches via blind index
- Industry-standard approach for searchable encryption

**Cons**:
- More complex implementation (two values per email)
- Blind index still theoretically vulnerable to frequency analysis
- Blind index leakage (can tell if two emails are the same)
- Additional storage overhead
- More complex to implement correctly

**Verdict**: ⚠️ Viable but unnecessarily complex for this use case

---

## Key Management Strategy

### Key Storage Options

#### Option A: Environment Variables

**Description**: Store encryption key in environment variable (e.g., `DPS_AUTH_API_EMAIL_ENCRYPTION_KEY`).

**Pros**:
- Simple to implement
- Keys not in code repository
- Standard practice for secrets
- Compatible with deployment platforms

**Cons**:
- Accessible to anyone with server access
- Visible in process listing on some systems
- Can be accidentally logged
- Requires secure environment variable storage in production

**Verdict**: ✅ **Recommended** for in-house solution

---

#### Option B: Configuration File

**Description**: Store encryption key in configuration file with restricted permissions.

**Pros**:
- Centralized key management
- Can be version controlled (excluding keys)
- Clear separation from code

**Cons**:
- File must be secured with proper permissions
- Risk of committing to version control
- Similar visibility concerns as environment variables

**Verdict**: ⚠️ Viable but environment variables are simpler

---

#### Option C: Key Derivation from Master Secret

**Description**: Derive email encryption key from master secret (e.g., using HKDF or simple concatenation).

**Pros**:
- Reduces number of secrets to manage
- Can derive different keys for different purposes
- Master secret can be stored via environment variable

**Cons**:
- More complex implementation
- Requires careful key derivation design
- Master secret compromise affects all keys

**Verdict**: ⚠️ Viable but adds unnecessary complexity for single-purpose key

---

### Recommended Key Management

**Approach**: Environment variable with 256-bit (32-byte) base64-encoded key

**Implementation Details**:

1. **Key Generation**:
   - Generate 32 cryptographically secure random bytes
   - Base64 encode for storage in environment variable
   - Decode in application before use

2. **Environment Variable Name**: `DPS_AUTH_API_EMAIL_ENCRYPTION_KEY`

3. **Key Validation**:
   - Validate at application startup
   - Ensure key is exactly 32 bytes after decoding
   - Fail startup if invalid

4. **Key Rotation** (if ever needed):
   - Since key rotation is out of scope per requirements, document that this requires data migration
   - Future consideration: maintain key versioning in database

**Security Considerations**:

- Never log the encryption key
- Add to `skip` list in `#[instrument]` macros
- Use `tracing` with appropriate filters to prevent accidental logging
- Document security requirements for production deployment

---

## Search Functionality Considerations

### Supported Searches with Deterministic Encryption

#### Equality Search (Exact Match)

**Description**: Check if a specific email address exists.

**Implementation**:
1. Hash search input using same deterministic encryption
2. Query database for encrypted value
3. If found, decrypt to confirm (optional validation)

**Performance**: O(log n) with index on encrypted column

**Example**:
```rust
// Encrypt search input
let encrypted_search = encrypt_email(search_input, &key);

// Query database
let result = sqlx::query!(
    "SELECT id, user_id, encrypted_email FROM email_addresses WHERE encrypted_email = ?",
    encrypted_search
)
.fetch_optional(pool)
.await?;
```

✅ **Fully supported** with good performance

---

#### Pattern/Prefix Search

**Description**: Find all emails matching a pattern (e.g., "@gmail.com", "user@").

**Challenge**: Deterministic encryption produces different ciphertext for different plaintexts. Pattern search on encrypted data is not feasible without complex homomorphic encryption (out of scope).

**Workaround Options**:

1. **Load and Decrypt in Memory**:
   - Fetch all email addresses
   - Decrypt in memory
   - Filter for pattern match
   - **Pros**: Simple to implement
   - **Cons**: Performance degrades with large datasets; all emails decrypted in memory

2. **Separate Searchable Columns** (Partially Compliant):
   - Store searchable components in plain text (e.g., domain only)
   - **Pros**: Fast pattern search
   - **Cons**: **Partial PII exposure**, reduced security

3. **Bloom Filters** (Advanced):
   - Use Bloom filters for approximate pattern matching
   - **Pros**: More secure than plain text searchable columns
   - **Cons**: Complex implementation; false positives possible

**Recommendation**: For initial implementation, **avoid pattern searches** or implement them as in-memory filters with appropriate limits and warnings. Document this limitation.

---

#### Range/Lexicographic Search

**Description**: Find emails between two values alphabetically.

**Challenge**: Not supported with deterministic encryption.

**Recommendation**: Avoid this use case. If needed, fetch all emails and filter in memory (with performance warnings).

---

#### Domain Search with Separated Local/Domain Storage

**Description**: Separate email addresses into two columns:
- `local_part` (before `@`): Encrypted with AES-256-GCM
- `domain` (after `@`): Stored as plain text

**Example**:
- Email: `john.doe@gmail.com`
- Stored as: `encrypted("john.doe")` | `gmail.com`

**Database Schema Concept**:
```sql
CREATE TABLE email_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    encrypted_local_part TEXT NOT NULL,
    domain TEXT NOT NULL,
    created_ts INTEGER NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_verified INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(id),
    UNIQUE(user_id, encrypted_local_part, domain)
);

-- Indexes for search performance
CREATE INDEX idx_email_addresses_domain ON email_addresses(domain);
CREATE INDEX idx_email_addresses_local_part ON email_addresses(encrypted_local_part);
```

**Search Implementation**:
```sql
-- Find all emails from gmail.com
SELECT id, user_id, encrypted_local_part, domain
FROM email_addresses
WHERE domain = 'gmail.com';

-- Find specific email (deterministic search)
SELECT id, user_id, encrypted_local_part, domain
FROM email_addresses
WHERE domain = 'gmail.com'
AND encrypted_local_part = encrypt('john.doe', key);
```

**Pros**:
- ✅ **Efficient domain search** - O(log n) with index on domain column
- ✅ **Exact email match still supported** - using encrypted local_part + domain
- ✅ **Minimal implementation complexity** - just split email into two parts
- ✅ **User expectations met** - domain filtering is a common requirement
- ✅ **Good performance** - domain search doesn't require decryption
- ✅ **Minimal storage overhead** - just one additional column

**Cons**:
- ⚠️ **Partial information leakage** - domain part stored in plain text
- ⚠️ **Domain frequency analysis** - attackers can see which domains are used
- ⚠️ **Cross-reference potential** - domain could be correlated with other data

---

### Compliance Analysis: Separated Local/Domain Storage

#### Europe (GDPR)

**Key Question**: Is the domain part considered personal data?

**Answer**: **No**, domains alone are not typically considered personal data under GDPR because:
- Domains are publicly available information (DNS, WHOIS, etc.)
- A domain name (e.g., "gmail.com") cannot identify an individual on its own
- GDPR Article 4(1) defines personal data as information relating to an *identified or identifiable natural person*

**GDPR Compliance Assessment**:
- **Local part (encrypted)**: ✅ Personal data, properly protected with strong encryption
- **Domain (plain text)**: ✅ Not personal data alone; public information
- **Combined (encrypted local + domain)**: ✅ Personal data protected; domain alone is public
- **Risk Assessment**: ⚠️ Low risk - domain leakage is minimal; local part is encrypted
- **Data Minimization**: ✅ Storing only necessary components (no unnecessary data)

**Guidance from GDPR Authorities**:
- Domain names are considered "indirect identifiers" at most
- Many organizations store domain statistics in plain text for analytics (common practice)
- The key concern is the local part, which is encrypted

**Compliance Verdict**: ✅ **Compliant** with GDPR, provided:
1. Local part is encrypted with strong encryption (AES-256-GCM)
2. Domain is truly public information (no corporate/hidden domains that could identify organizations)
3. Appropriate access controls are in place
4. Data Protection Impact Assessment (DPIA) is performed if processing large volumes

---

#### United States

**Key Question**: Is the domain part considered PII under US state privacy laws?

**Answer**: **No**, domain names are not considered PII under:
- **CCPA/CPRA (California)**: Domain alone cannot identify a person
- **Other state laws**: Consistent interpretation - domains are public information

**Compliance Assessment**:
- **Local part (encrypted)**: ✅ PII protected with encryption
- **Domain (plain text)**: ✅ Not PII; publicly available
- **Combined (encrypted local + domain)**: ✅ PII protected

**US Privacy Law Verdict**: ✅ **Compliant** with all US state privacy laws

---

#### Canada (PIPEDA)

**Key Question**: Is the domain part considered personal information under PIPEDA?

**Answer**: **No**, domain names are not personal information on their own:
- PIPEDA defines personal information as information about an identifiable individual
- Domain names alone cannot identify individuals
- Domains are public information

**Compliance Assessment**:
- **Local part (encrypted)**: ✅ Personal information protected with encryption
- **Domain (plain text)**: ✅ Not personal information alone
- **Combined (encrypted local + domain)**: ✅ Personal information protected

**PIPEDA Compliance Verdict**: ✅ **Compliant** with PIPEDA

---

#### Brazil (LGPD)

**Key Question**: Is the domain part considered personal data under LGPD?

**Answer**: **No**, domain names are not personal data alone:
- LGPD Article 5(I) defines personal data as information that can identify an individual
- Domain names cannot identify individuals on their own
- Domains are public information

**Compliance Assessment**:
- **Local part (encrypted)**: ✅ Personal data protected with encryption
- **Domain (plain text)**: ✅ Not personal data alone
- **Combined (encrypted local + domain)**: ✅ Personal data protected

**LGPD Compliance Verdict**: ✅ **Compliant** with LGPD

---

### Security Assessment

**Information Leakage**:
- **What's exposed**: Only domain names (e.g., "gmail.com", "outlook.com")
- **What's not exposed**: Actual email local parts, user identities
- **Risk Level**: 🟡 **Low** - domains are public information anyway

**Attack Vectors**:

1. **Domain Frequency Analysis**:
   - **Risk**: Attacker can see which domains are most used (e.g., "70% of users use gmail.com")
   - **Impact**: Minimal - this information is often public knowledge or can be inferred from other sources
   - **Mitigation**: Not needed; acceptable risk

2. **Cross-Reference with Other Data**:
   - **Risk**: Attacker combines domain data with other leaked data to identify users
   - **Impact**: 🟡 **Low to Medium** - depends on what other data is available
   - **Mitigation**: Ensure domain data is not combined with other identifying data in analytics; keep database access restricted

3. **Pattern Matching on Encrypted Local Part**:
   - **Risk**: If using deterministic encryption, attacker could potentially perform frequency analysis on local parts
   - **Impact**: 🟡 **Low** - requires access to encrypted data; mitigated by strong encryption (AES-256-GCM)
   - **Mitigation**: Consider adding user ID to encryption context if needed

**Comparison with Full Encryption**:

| Aspect | Full Encryption | Separated Local/Domain |
|--------|-----------------|------------------------|
| Domain search | ❌ Not supported | ✅ Full support (indexed) |
| Exact email match | ✅ Full support | ✅ Full support |
| Information leakage | 🟢 None | 🟡 Domain names only |
| Security level | 🟢 Excellent | 🟢 Good (local part encrypted) |
| Compliance | ✅ Excellent | ✅ Compliant |
| Implementation complexity | 🟢 Simple | 🟢 Simple |
| User utility | 🟡 Limited | 🟢 High |

**Security Verdict**: ✅ **Acceptable** security posture for most use cases, provided:
1. Local part is encrypted with AES-256-GCM
2. Access to database is properly restricted
3. No unnecessary aggregation or analytics on domain data

---

### Real-World Examples and Industry Practice

Many organizations use this approach:

1. **Email Service Providers**: Store domain statistics separately for analytics
2. **Authentication Systems**: Often separate local and domain for routing and management
3. **Marketing Platforms**: Track domain usage patterns in plain text for segmentation

**Industry Standards**:
- This is considered **acceptable practice** in the industry
- No major privacy guidelines explicitly prohibit storing domains in plain text
- The key concern is protecting the identifying information (local part)

---

### Recommendation

**Verdict**: ✅ **Recommended** for domain search functionality

**Justification**:
1. **Compliant** with all analyzed jurisdictions (Europe, US, Canada, Brazil)
2. **Acceptable security** - only public domain information is exposed in plain text
3. **Excellent performance** - domain searches are indexed and efficient
4. **User expectations met** - domain filtering is a common and useful feature
5. **Simple implementation** - minimal complexity over fully encrypted approach
6. **Industry practice** - widely used and accepted approach

---

#### Alternative: Both Parts Encrypted with Deterministic Encryption

**Description**: Encrypt BOTH local part AND domain with deterministic AES-256-GCM, keeping them in separate columns. This maintains domain search capability while providing maximum security.

**Example**:
- Email: `john.doe@gmail.com`
- Stored as: `encrypted("john.doe")` | `encrypted("gmail.com")`

**How It Works**:
- **Deterministic encryption**: Same plaintext + same key = same ciphertext
- `encrypt("gmail.com", key)` always produces the same encrypted value
- To search, encrypt the search term using the same deterministic function
- Database comparison: `WHERE encrypted_domain = encrypt("gmail.com", key)`
- No decryption needed for searches

**Database Schema Concept**:
```sql
CREATE TABLE email_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    encrypted_local_part TEXT NOT NULL,
    encrypted_domain TEXT NOT NULL,
    created_ts INTEGER NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_verified INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(id),
    UNIQUE(user_id, encrypted_local_part, encrypted_domain)
);

-- Indexes for search performance
CREATE INDEX idx_email_addresses_encrypted_domain ON email_addresses(encrypted_domain);
CREATE INDEX idx_email_addresses_local_part ON email_addresses(encrypted_local_part);
```

**Search Implementation**:
```sql
-- Find all emails from gmail.com (deterministic search)
-- First, encrypt the search term using the same deterministic encryption
-- Then query the database
SELECT id, user_id, encrypted_local_part, encrypted_domain
FROM email_addresses
WHERE encrypted_domain = encrypt('gmail.com', key);

-- Find specific email (deterministic search on both parts)
SELECT id, user_id, encrypted_local_part, encrypted_domain
FROM email_addresses
WHERE encrypted_domain = encrypt('gmail.com', key)
AND encrypted_local_part = encrypt('john.doe', key);
```

**Pros**:
- ✅ **Maximum security** - ALL PII encrypted, including domains
- ✅ **Efficient domain search** - O(log n) with index on encrypted_domain column
- ✅ **Exact email match supported** - using both encrypted columns
- ✅ **Minimal information leakage** - No domain visibility in database
- ✅ **Compliant for all domains** - Personal domains (.name, .me) fully protected
- ✅ **Simple implementation** - same complexity as current approach
- ✅ **No external dependencies** - in-house solution
- ✅ **Analytics require decryption** - Domain statistics require decrypting (security feature)
- ✅ **No frequency analysis visibility** - Attacker can't see domain patterns without decryption key

**Cons**:
- ⚠️ **Same theoretical vulnerability** - frequency analysis possible on BOTH columns
   - Attacker with database access could see "70 users have encrypted_domain = X"
   - However, this is actually LESS risky than plain text domains!
   - Attacker can't tell which domain without knowing encryption key
- ⚠️ **Analytics complexity** - Domain popularity requires decrypting data
   - Cannot easily query "which domains are most popular?" without decryption
   - Would need to decrypt all domains for analytics
- ⚠️ **Search parameter encryption** - Search terms must be encrypted before query
   - Application must encrypt search parameters (not a real con, just requirement)

**Comparison with Plain Text Domain Storage**:

| Aspect | Plain Text Domain | Encrypted Domain (Deterministic) |
|---------|-------------------|--------------------------------|
| Security | 🟡 Good | 🟢 Excellent |
| Domain search | ✅ Simple (WHERE domain = 'gmail.com') | ✅ Simple (WHERE domain = encrypt('gmail.com', key)) |
| Information leakage | 🟡 Domain visible in database | 🟢 No leakage |
| Personal domains (.name, .me) | ⚠️ PII exposed | ✅ Fully protected |
| Analytics (domain stats) | ✅ Easy (SELECT domain, COUNT(*)) | ⚠️ Requires decryption |
| Performance | 🟢 Excellent (indexed) | 🟢 Excellent (indexed) |
| Implementation complexity | 🟢 Simple | 🟢 Simple |
| Compliance | ✅ Compliant | ✅ More compliant (all PII encrypted) |
| Frequency analysis risk | 🟡 Attacker sees "gmail.com: 70 users" | 🟡 Attacker sees "X7f9a...: 70 users" (but can't identify domain) |

**Security Assessment: Frequency Analysis**

**Theoretical Vulnerability** (Both Approaches):
- Deterministic encryption allows frequency analysis: "which encrypted value appears most often?"

**Plain Text Domains**:
```sql
-- Attacker queries database
SELECT domain, COUNT(*) as count
FROM email_addresses
GROUP BY domain
ORDER BY count DESC;

-- Result: attacker sees
-- gmail.com (70 users)
-- outlook.com (30 users)
-- company.com (15 users)
-- john.smith.name (1 user)

-- Attacker has DIRECT visibility of domain patterns!
```

**Encrypted Domains**:
```sql
-- Attacker queries database
SELECT encrypted_domain, COUNT(*) as count
FROM email_addresses
GROUP BY encrypted_domain
ORDER BY count DESC;

-- Result: attacker sees
-- X7f9a2b3c... (70 users)
-- Y8e4f1d9a... (30 users)
-- Z1f2g3e5b... (15 users)
-- A2d3c4f6e... (1 user)

-- Attacker has NO IDEA what these encrypted values mean!
-- Without encryption key, they can't tell which domains are popular.
```

**Key Insight**: Encrypted domains provide the SAME theoretical frequency analysis capability as plain text domains, but with **BETTER protection** because the attacker can't identify the domains without the encryption key.

**Real-World Security Assessment**:

**Scenario**: Database breach occurs

**With Plain Text Domains**:
- ✅ Attacker sees: `john.doe@gmail.com` in database
- ✅ Attacker sees: 70 users have `@gmail.com`
- ✅ Attacker sees: 1 user has `@john.smith.name` (personally identifiable!)
- Risk: **HIGH** - Direct PII exposure

**With Encrypted Domains**:
- ✅ Attacker sees: `encrypted("john.doe")` | `encrypted("gmail.com")`
- ⚠️ Attacker sees: 70 users have `encrypted("gmail.com")`
- ⚠️ Attacker sees: 1 user has `encrypted("john.smith.name")`
- ❌ Attacker can't decrypt without encryption key
- Risk: **LOW** - Encrypted data without key

**Conclusion**: Encrypted domains are SIGNIFICANTLY more secure than plain text domains in breach scenarios.

---

#### Recommendation: Both Parts Encrypted (Alternative)

**Verdict**: ✅ **Strongly Recommended** as an alternative to the primary recommendation

**Why this might be better**:
1. **Maximum security**: All PII encrypted, no exceptions
2. **Personal domain protection**: .name, .me domains fully encrypted
3. **Better breach protection**: Domain patterns not visible without key
4. **Same performance**: Domain search still efficient with indexing
5. **Same complexity**: Deterministic encryption for both parts
6. **Analytics as security feature**: Domain statistics require decryption

**When to choose this approach**:
- High-security applications (financial, healthcare, government)
- Applications with many personal domains (.name, .me)
- Regulatory requirements demand maximum encryption
- Where "analytics" feature is not critical

**Trade-off acceptance**:
- Slightly more complex domain analytics (requires decryption)
- Same frequency analysis theoretical vulnerability (but better protected than plain text)
- Slightly more processing (encrypt search terms)

**Implementation Priority** (if choosing this approach):
1. Encrypt both local_part AND domain with deterministic AES-256-GCM
2. Create indexes on both encrypted columns
3. Implement search term encryption in application layer
4. Domain analytics require decryption (explicit, controlled process)

---

### Compliance Analysis: Separated Local/Domain Storage

#### Original Approach: Local Encrypted + Domain Plain Text

**Compliance Verdict**: ✅ **Compliant** with all analyzed jurisdictions (EU, US, Canada, Brazil)

**Implementation Requirements**:
1. Encrypt local part with AES-256-GCM
2. Store domain as plain text
3. Create appropriate database indexes
4. Document domain storage in privacy policy
5. Consider DPIA if processing large volumes of user data
6. Ensure database access controls are in place

---

### Advanced Analysis: Personal and Corporate Domains

**Overview**: The analysis above assumes domains are "public" email providers (gmail.com, outlook.com, etc.). However, organizations and individuals may use their own domains, which introduces additional compliance considerations.

#### Domain Classification

**1. Public Email Provider Domains**

**Examples**: `gmail.com`, `outlook.com`, `yahoo.com`, `hotmail.com`, `icloud.com`

**Characteristics**:
- Millions of users per domain
- Cannot identify individuals or organizations
- Publicly known and accessible
- Domain alone provides no identifying information

**Compliance Status**: ✅ **Not PII** - Domain is public information, no identifying power

**Security Impact**: 🟢 **None** - Acceptable to store in plain text

---

**2. Corporate/Business Domains**

**Examples**: `dimensionalpocket.com`, `company.com`, `startup.io`, `business.co`

**Characteristics**:
- Domain typically associated with an organization
- Can identify companies/organizations
- May be unique or have few users
- Could reveal business relationships or partnerships

**Compliance Status**: ⚠️ **Context-Dependent**

**Analysis**:

| Jurisdiction | Corporate Domain Status | Reasoning |
|--------------|------------------------|-------------|
| Europe (GDPR) | **Organizational data, not personal** | GDPR protects "natural persons", not legal entities. Corporate domains identify organizations, not individuals. |
| US (CCPA/CPRA) | **Not PII** | CCPA defines PII as information about "individuals". Corporate domains identify businesses. |
| Canada (PIPEDA) | **Business information** | PIPEDA distinguishes between personal information and business information. |
| Brazil (LGPD) | **Personal data of legal entities** | LGPD has specific provisions for personal data of legal entities. |

**Key Insight**: Corporate domains are typically **not PII for individuals** because they identify **organizations**, not people. However, they may be considered **sensitive business information**.

**Security Impact**: 🟡 **Low to Medium** - Business data, not personal data, but may be sensitive

**Risks**:
1. **Competitive intelligence**: Could reveal customer base, partnerships, or supplier relationships
2. **Business profiling**: Competitors could analyze which organizations use your service
3. **Social engineering**: Domain names could be used in targeted phishing attacks

**Mitigation Strategies**:
1. **Access controls**: Restrict database access to authorized personnel only
2. **Audit logging**: Track who accesses domain data
3. **Data classification**: Classify corporate domains as "confidential business information"
4. **Aggregation limits**: Implement policies to prevent large-scale domain analysis
5. **Contextual disclosure**: Only expose domain data when necessary for legitimate business purposes

**Recommendation**: For most applications, storing corporate domains in plain text is **acceptable**, but treat as business-confidential rather than public data.

---

**3. Personal Domains (.name, .me, etc.)**

**Examples**:
- Personal TLDs: `john.name`, `jane.smith.name`, `firstname.lastname.name`
- Personal branding: `john.me`, `jane.me`, `branded.me`
- Personalized domains: `firstname-lastname.com`, `personal-blog.com`

**Characteristics**:
- Explicitly designed for individuals
- Domain often contains or relates to a person's name
- Can identify specific individuals
- May have only one user (the domain owner)

**Compliance Status**: ⚠️ **Potentially PII** - Context-dependent

**GDPR Analysis**:

**Definition**: Under GDPR Article 4(1), personal data is "any information relating to an identified or identifiable natural person."

**When personal domains ARE PII**:
- Domain contains person's name: `john.smith.name` → **Identifies John Smith**
- Domain is unique to individual: `john.doe.com` → **Identifies John Doe**
- Domain can be cross-referenced with other data to identify someone

**When personal domains MAY NOT be PII**:
- Generic personal brand: `mydigitalidentity.me` (no identifying info)
- Shared family domain: `smith-family.com` (identifies family, not necessarily specific individual)
- Anonymized personal domain: `randomperson.net`

**Key Factors**:
1. **Uniqueness**: Is the domain unique to one person?
2. **Identifiability**: Can the domain identify someone on its own?
3. **Cross-reference**: Can it be combined with other data to identify someone?

**Compliance Assessment Table**:

| Domain Type | Example | PII Status | Reasoning |
|--------------|----------|--------------|-------------|
| `john.smith.name` | ✅ **PII** | Contains full name, identifies individual |
| `john.doe.me` | ✅ **PII** | Unique to John Doe, identifies individual |
| `smith.family.com` | ⚠️ **Partial PII** | Identifies family, not specific individual |
| `mybrand.me` | 🟡 **Possibly PII** | Depends on cross-reference with other data |
| `generic.name` | ❌ **Not PII** | Generic, no identifying information |

**Other Jurisdictions**:

| Jurisdiction | Personal Domain PII Status |
|--------------|--------------------------|
| US (CCPA) | **PII if identifiable** - Same contextual analysis as GDPR |
| Canada (PIPEDA) | **Personal information if identifiable** - Contextual approach |
| Brazil (LGPD) | **Personal data if identifiable** - Similar to GDPR |

**Security Impact**: 🔴 **High** - Can directly identify individuals

**Risks**:
1. **Direct identification**: Domain reveals person's identity without needing local part
2. **Profiling**: Can build profiles of individuals based on domain choices
3. **Cross-reference**: Can be linked to other data (social media, public records)
4. **Targeted attacks**: Enables personalized phishing or social engineering

**Real-World Example**:
- Email: `john.smith@john.smith.name`
- Local part encrypted: `encrypted("john.smith")`
- Domain stored as plain text: `john.smith.name`

**Problem**: Even without decrypting local part, the domain `john.smith.name` alone identifies John Smith.

**Mitigation Strategies**:

**Option 1: Encrypt Personal Domains**

**Implementation**:
```sql
-- Add domain_encryption_flag column
ALTER TABLE email_addresses ADD COLUMN domain_is_encrypted INTEGER DEFAULT 0;

-- For personal domains, encrypt them too
-- For public domains (gmail.com, etc.), store plain text
```

**Logic**:
1. On email address registration, classify domain:
   - Check domain against list of known personal TLDs (.name, .me)
   - Check if domain contains person's name pattern
   - Use domain reputation/lookup to determine if it's personal

2. For personal domains:
   - Encrypt domain along with local part
   - Store as encrypted_domain column
   - Set domain_is_encrypted = 1

3. For public/business domains:
   - Store domain as plain text
   - Set domain_is_encrypted = 0

**Trade-offs**:
- ✅ **Pros**: Better security for personal domains
- ✅ **Compliant**: Addresses PII concerns for personal domains
- ❌ **Cons**: Cannot perform domain search on encrypted domains
- ❌ **Cons**: More complex implementation
- ❌ **Cons**: Requires domain classification logic

---

**Option 2: Domain Hashing (Blind Index)**

**Implementation**:
```sql
CREATE TABLE email_addresses (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    encrypted_local_part TEXT NOT NULL,
    domain TEXT NOT NULL,
    domain_hash TEXT NOT NULL,  -- Hash for search without exposing
    created_ts INTEGER,
    is_primary INTEGER DEFAULT 0,
    is_verified INTEGER DEFAULT 0,
    UNIQUE(user_id, encrypted_local_part, domain)
);

CREATE INDEX idx_email_addresses_domain_hash ON email_addresses(domain_hash);
```

**Logic**:
1. For personal domains:
   - Store domain as plain text (for display)
   - Create domain_hash = SHA-256(domain + salt)
   - Use hash for search comparisons

2. For queries:
   - Hash search domain: SHA-256(search_domain + salt)
   - Query WHERE domain_hash = ?
   - If match found, reveal domain only to authorized users

**Trade-offs**:
- ✅ **Pros**: Can still search domains (via hash)
- ✅ **Pros**: Hash doesn't directly reveal domain
- ✅ **Pros**: Better than plain text for privacy
- ❌ **Cons**: Still exposes domain when displaying email
- ❌ **Cons**: Hash doesn't protect against rainbow tables if salt is weak
- ❌ **Cons**: More complex than plain text

---

**Option 3: Store All Domains as Reference (Recommended)**

**Philosophy**: Treat all domains consistently and rely on local part encryption for privacy.

**Implementation**:
```sql
-- Simple approach: store all domains as plain text
CREATE TABLE email_addresses (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    encrypted_local_part TEXT NOT NULL,
    domain TEXT NOT NULL,
    created_ts INTEGER,
    is_primary INTEGER DEFAULT 0,
    is_verified INTEGER DEFAULT 0,
    UNIQUE(user_id, encrypted_local_part, domain)
);

CREATE INDEX idx_email_addresses_domain ON email_addresses(domain);
```

**Rationale**:
1. **Local part encryption is primary protection** - This is where the real PII lives
2. **Domain exposure is minimal risk** - Even for personal domains:
   - Most personal domains are publicly discoverable (DNS, WHOIS before GDPR)
   - Email address itself (encrypted local part) still protected
   - Context matters: domain alone doesn't reveal the actual email address
3. **Simplicity favors security** - Complex implementations introduce bugs
4. **User experience** - Domain search is valuable functionality
5. **Transparency** - Document domain storage in privacy policy

**Documentation Requirement**:
```
Privacy Policy Section:

"We store email addresses in our database with the local part (before @) encrypted
and the domain part (after @) stored as plain text. This allows us to
provide efficient domain-based search functionality while protecting the identifying
information in your email address.

For personal domains (e.g., .name, .me domains), we acknowledge that
the domain may be personally identifiable. However, the local part of your
email address remains encrypted with AES-256-GCM, ensuring that your
complete email address cannot be reconstructed without proper authorization."
```

**Trade-offs**:
- ✅ **Pros**: Simple, maintainable, less error-prone
- ✅ **Pros**: Domain search functionality preserved
- ✅ **Pros**: Consistent with industry practice
- ✅ **Pros**: Local part encryption provides strong protection
- ⚠️ **Cons**: Personal domains visible in database
- ⚠️ **Cons**: Some information leakage for personal domains

---

#### Comprehensive Compliance Assessment

**Summary Table**:

| Domain Type | Storage Approach | Compliance Status | Risk Level | Recommended? |
|--------------|------------------|-------------------|--------------|----------------|
| Public providers (gmail.com, etc.) | Plain text | ✅ Compliant | 🟢 None | ✅ Yes |
| Corporate domains (company.com) | Plain text | ✅ Compliant (not individual PII) | 🟡 Low-Medium | ✅ Yes (with access controls) |
| Personal domains (.name, .me) | Plain text | ⚠️ Compliant with transparency | 🔴 High | ✅ Yes (with documentation) |
| Personal domains (.name, .me) | Encrypted | ✅ Compliant | 🟢 Low | ⚠️ Maybe (loses search capability) |

---

#### Practical Recommendations

**For Most Applications**:

**Recommendation**: **Option 3 - Store All Domains as Reference**

**Why**:
1. **Local part encryption is the primary protection** for PII
2. **Domain exposure is acceptable** for the value provided (search functionality)
3. **Simpler is more secure** than complex implementations
4. **Industry standard** approach
5. **Compliant** with all jurisdictions with proper documentation

**Required Actions**:
1. ✅ Encrypt local parts with AES-256-GCM
2. ✅ Store all domains as plain text
3. ✅ Implement database access controls
4. ✅ Document domain storage approach in privacy policy
5. ✅ Audit logging for domain access
6. ✅ Consider domain classification for internal risk assessment

**For High-Security Applications**:

**Recommendation**: **Option 1 - Encrypt Personal Domains**

**When to use**:
- Financial services
- Healthcare (HIPAA)
- Government applications
- Law enforcement
- Applications with exceptionally sensitive user bases

**Trade-off acceptance**: Lose domain search for personal domains in exchange for maximum privacy.

---

**Considerations for Sensitive Domains**:
- If using corporate/private domains that could identify organizations, additional analysis may be needed
- For public email providers (gmail.com, outlook.com, yahoo.com, etc.), no issues
- For personal domains (.name, .me), transparency in privacy policy is sufficient
- Access controls and audit logging are critical for corporate and personal domains

---

### Search Strategy Summary

**Two Viable Approaches**:

#### Approach 1: Local Encrypted + Domain Plain Text (Primary Recommendation)

| Search Type | Support | Performance | Security | Notes |
|-------------|---------|-------------|----------|--------|
| Exact email match | ✅ Full | Excellent (indexed) | Excellent | Secure |
| Domain search (public providers) | ✅ Full | Excellent (indexed) | Excellent | Compliant |
| Domain search (corporate domains) | ✅ Full | Excellent (indexed) | Good | Business-confidential |
| Domain search (personal domains) | ✅ Full | Excellent (indexed) | Medium | PII exposure; documented |
| Pattern/Prefix (e.g., "user@") | ⚠️ Limited (in-memory) | Poor (scales with dataset) | Good | Requires decryption |
| Range/Lexicographic | ❌ Not supported | N/A | N/A | - |

---

#### Approach 2: Both Parts Encrypted with Deterministic Encryption (Alternative, More Secure)

| Search Type | Support | Performance | Security | Notes |
|-------------|---------|-------------|----------|--------|
| Exact email match | ✅ Full | Excellent (indexed) | Excellent | Secure |
| Domain search (public providers) | ✅ Full | Excellent (indexed) | Excellent | Fully encrypted |
| Domain search (corporate domains) | ✅ Full | Excellent (indexed) | Excellent | Fully encrypted |
| Domain search (personal domains) | ✅ Full | Excellent (indexed) | Excellent | Fully encrypted |
| Pattern/Prefix (e.g., "user@") | ⚠️ Limited (in-memory) | Poor (scales with dataset) | Excellent | Requires decryption |
| Range/Lexicographic | ❌ Not supported | N/A | N/A | - |

**Key Difference**: Approach 2 provides maximum security by encrypting domains too, with same search capability via deterministic encryption.

---

## Backup Strategy Analysis

### Current Proposal

**Description**: S3 backup of password-protected zipped database file.

### ZIP Encryption Analysis

**Standard ZIP Encryption**:
- Traditional ZIP 2.0 encryption (AES-128) - **WEAK**, vulnerable to known-plaintext attacks
- ZIP 2.0 + compression metadata leaks (filenames visible)
- Brute-force attacks are feasible
- **Non-compliant** with modern security standards

**Modern ZIP Encryption** (WinZip AES):
- AES-256 encryption available
- Better security than traditional ZIP
- Still considered less secure than dedicated file encryption
- Not all ZIP tools support this format

**GDPR Compliance**:
- ZIP 2.0: ❌ Non-compliant (weak encryption)
- WinZip AES-256: ⚠️ Borderline (acceptable but not recommended)

### Compliance Requirements for Backups

All jurisdictions require:
- **Encryption at rest** (for PII)
- **Secure transmission** (encrypted in transit)
- **Access controls** (who can access backups)

### S3 Backup Options

#### Option A: ZIP + S3 Server-Side Encryption (SSE)

**Description**:
1. Password-protect ZIP file (with AES-256)
2. Upload to S3 with SSE-S3 or SSE-KMS

**Pros**:
- Double encryption (ZIP + S3)
- S3 handles key management for SSE
- Simple to implement

**Cons**:
- ZIP password must be managed separately
- ZIP encryption still has weaknesses
- Complexity of two encryption layers

**Verdict**: ⚠️ Viable but more complex than needed

---

#### Option B: S3 Server-Side Encryption Only

**Description**: Upload database to S3 with SSE-S3 or SSE-KMS (no ZIP password).

**Pros**:
- Simple implementation
- Strong encryption (AES-256)
- S3 handles key management
- AWS KMS provides detailed audit logging
- Compliant with GDPR, LGPD, etc.

**Cons**:
- Requires external service (S3)
- **Violates in-house preference**

**Verdict**: ❌ Not suitable due to external service requirement

---

#### Option C: In-House File Encryption + Local/Simple Storage

**Description**:
1. Encrypt database file using AES-256-GCM
2. Store encrypted file on local filesystem or simple network storage
3. Manage encryption key via environment variable (same as application)

**Pros**:
- In-house solution
- Strong encryption (AES-256-GCM)
- No external dependencies
- Full control over key management
- Can implement in Rust using existing `aes-gcm` crate

**Cons**:
- Requires backup storage infrastructure
- Manual key management and rotation (if needed)
- No built-in redundancy (needs separate solution)

**Verdict**: ✅ **Recommended** for in-house approach

---

### Recommended Backup Strategy

**Approach**: File-level encryption with S3 storage (using AWS only as storage, not encryption)

**Implementation**:

1. **Encryption Layer** (In-House):
   - Use AES-256-GCM to encrypt entire database file
   - Key stored in environment variable
   - Generate random nonce for each backup

2. **Storage** (S3):
   - Upload encrypted file to S3
   - Use S3 only as encrypted blob storage (no SSE)
   - S3 access controls and IAM policies

3. **Backup Process**:
   ```bash
   # Pseudocode
   backup_db() {
       # 1. Stop writes (or use SQLite backup API)
       # 2. Copy database to temp location
       # 3. Encrypt with AES-256-GCM using environment key
       # 4. Upload encrypted file to S3
       # 5. Clean up temp files
   }
   ```

4. **Restore Process**:
   ```bash
   # Pseudocode
   restore_db() {
       # 1. Download encrypted file from S3
       # 2. Decrypt with AES-256-GCM using environment key
       # 3. Replace database file
   }
   ```

**Compliance Assessment**:

| Jurisdiction | Compliant? | Notes |
|--------------|------------|-------|
| Europe (GDPR) | ✅ Yes | AES-256-GCM meets encryption requirements |
| US | ✅ Yes | NIST-approved AES-256 |
| Canada (PIPEDA) | ✅ Yes | Strong encryption demonstrates due diligence |
| Brazil (LGPD) | ✅ Yes | Appropriate security measure |

**Security Benefits**:
- Double-layer protection (file encryption + S3 access controls)
- File encrypted before leaving server
- S3 never sees plaintext data
- Full in-house control over encryption key
- Can rotate encryption key independently of S3

---

## Recommended Approach

### Overview: Two Viable Approaches

This analysis identifies two compliant approaches for storing email addresses, each with different trade-offs:

| Approach | Local Part | Domain | Security | Complexity |
|----------|--------------|--------|----------|-------------|
| **1. Primary** (Recommended for most apps) | Encrypted (AES-256-GCM) | Plain text | Good | Simple |
| **2. Alternative** (More secure, recommended for high-security) | Encrypted (AES-256-GCM) | Encrypted (AES-256-GCM, deterministic) | Excellent | Simple |

**Both approaches**:
- ✅ Support exact email match
- ✅ Support domain search (most common use case)
- ✅ Use deterministic encryption for searchable fields
- ✅ Simple, in-house implementation
- ✅ Compliant with all jurisdictions (EU, US, Canada, Brazil)

**Key difference**: Approach 2 encrypts domains too, providing maximum security for personal domains (.name, .me).

---

### Approach 1: Local Encrypted + Domain Plain Text (Primary Recommendation)

**Use case**: Most applications, general-purpose authentication systems, SaaS platforms.

**Why this approach**:
- Local part encrypted with AES-256-GCM (strong encryption)
- Domain stored as plain text:
  - Public providers (gmail.com, etc.): Public information, fully compliant
  - Corporate domains (company.com): Business information, compliant with proper access controls
  - Personal domains (.name, .me): Potentially PII; addressed with transparency and documentation
- Supports both exact match and domain searches
- Simple implementation
- In-house solution
- Compliant with all analyzed jurisdictions (EU, US, Canada, Brazil) with proper documentation
- Can leverage existing `aes-gcm` and `base64` dependencies
- Domain search meets user expectations and is industry standard

**Database Schema** (simplified):
```sql
CREATE TABLE email_addresses (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    encrypted_local_part TEXT NOT NULL,
    domain TEXT NOT NULL,  -- PLAIN TEXT
    created_ts INTEGER NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_verified INTEGER NOT NULL DEFAULT 0,
    UNIQUE(user_id, encrypted_local_part, domain)
);

CREATE INDEX idx_email_addresses_domain ON email_addresses(domain);
```

**Domain Search**:
```sql
-- Simple, efficient
SELECT * FROM email_addresses WHERE domain = 'gmail.com';
```

**Trade-offs**:
- ✅ Simpler domain analytics (no decryption needed)
- ✅ Industry standard approach
- ⚠️ Personal domains (.name, .me) visible in plain text (documented in privacy policy)

---

### Approach 2: Both Parts Encrypted with Deterministic Encryption (More Secure)

**Use case**: High-security applications (financial, healthcare, government), or systems with many personal domains (.name, .me).

**Why this approach**:
- **Maximum security**: ALL PII encrypted, including domains
- **Same search capability**: Domain search works via deterministic encryption
- **No exceptions**: Personal domains (.name, .me) fully protected
- **Better breach protection**: Domain patterns not visible without encryption key
- Same complexity as Approach 1
- In-house solution
- More compliant than Approach 1 (all PII encrypted)

**Database Schema** (simplified):
```sql
CREATE TABLE email_addresses (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    encrypted_local_part TEXT NOT NULL,
    encrypted_domain TEXT NOT NULL,  -- ENCRYPTED (deterministic)
    created_ts INTEGER NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_verified INTEGER NOT NULL DEFAULT 0,
    UNIQUE(user_id, encrypted_local_part, encrypted_domain)
);

CREATE INDEX idx_email_addresses_encrypted_domain ON email_addresses(encrypted_domain);
```

**Domain Search** (requires encryption of search term):
```rust
// Rust example
fn search_emails_by_domain(domain: &str) -> Result<Vec<EmailAddress>> {
    let encrypted_domain = encrypt_deterministic(domain.as_bytes(), &encryption_key)?;
    sqlx::query!(
        "SELECT id, user_id, encrypted_local_part, encrypted_domain
         FROM email_addresses
         WHERE encrypted_domain = ?",
        encrypted_domain
    )
    .fetch_all(pool)
    .await
}

// SQL
SELECT * FROM email_addresses WHERE encrypted_domain = encrypt('gmail.com', key);
```

**Trade-offs**:
- ✅ Maximum security for all PII (including personal domains)
- ✅ Domain search still efficient
- ✅ Better breach protection (domain patterns encrypted)
- ⚠️ Domain analytics require decryption (security feature, not bug)
- ⚠️ Slightly more complex search (encrypt search terms)
- Same theoretical frequency analysis vulnerability as Approach 1 (but better protected)

**Implementation Note**: Only difference from Approach 1 is:
- Domain column is encrypted instead of plain text
- Search terms must be encrypted before query (simple wrapper function)
- Everything else is identical

---

### Key Management: Environment Variables (Both Approaches)

**Why**:
- Simple and standard
- No external dependencies
- Compatible with existing project patterns (see `DPS_AUTH_API_SESSION_SECRET`)
- Works well with containerization and cloud deployment

**Environment Variables**:
- `DPS_AUTH_API_EMAIL_ENCRYPTION_KEY`: Base64-encoded 256-bit key for email encryption

---

### Search: Exact Match + Domain Search (Both Approaches)

**Why**:
- Exact email match for primary use cases
- Domain search for filtering and analytics (e.g., "all @gmail.com users")
- Excellent performance with proper indexing
- Pattern/substring searches not supported (documented limitation)
- Domain search works efficiently with either approach:
  - **Approach 1**: Direct WHERE domain = 'gmail.com'
  - **Approach 2**: WHERE encrypted_domain = encrypt('gmail.com', key)

### Backup: In-House File Encryption with S3 Storage

**Why**:
- Strong encryption (AES-256-GCM) before upload
- S3 used only as storage, not encryption service
- Double-layer security (file encryption + S3 access controls)
- In-house control over encryption keys
- Compliant with all jurisdictions
- Avoids weak ZIP encryption

---

## Implementation Considerations

### Potential Rust Crates

**Already in project**:
- `aes-gcm` (v0.10) - ✅ Can use for email encryption
- `base64` (v0.22) - ✅ Can use for key encoding

**Additional crates to consider**:
- `hkdf` - If key derivation is needed in the future
- `rand` (already in project) - For nonce/key generation
- `argon2` (already in project) - For future password-based key derivation if needed

### Database Schema

**Conceptual structure** (implementation details not in scope):
```sql
CREATE TABLE email_addresses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    encrypted_local_part TEXT NOT NULL,
    domain TEXT NOT NULL,
    created_ts INTEGER NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_verified INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(id),
    UNIQUE(user_id, encrypted_local_part, domain)
);

-- Indexes for search performance
CREATE INDEX idx_email_addresses_domain ON email_addresses(domain);
CREATE INDEX idx_email_addresses_local_part ON email_addresses(encrypted_local_part);
```

### Performance Considerations

- **Encryption overhead**: Minimal with AES-256-GCM (hardware-accelerated on modern CPUs)
- **Search performance**: Excellent for exact match (indexed), excellent for domain search (indexed), poor for pattern/substring search (in-memory filter)
- **Storage overhead**: AES-GCM adds ~16 bytes for authentication tag, ~12 bytes for nonce (if stored), plus additional domain column
- **Database size**: ~25-35% increase over plain text storage (due to encrypted local_part + domain separation)

### Security Best Practices

1. **Key Security**:
   - Never log encryption keys
   - Add to `skip` list in `#[instrument]` macros
   - Restrict environment variable access at OS level
   - Use strong random key generation

2. **Encryption Implementation**:
   - Use AES-256-GCM (authenticated encryption)
   - Generate random nonce for each encryption (if not deterministic)
   - For deterministic encryption, ensure nonce handling is correct
   - Validate encryption/decryption operations

3. **Error Handling**:
   - Fail securely (decrypt errors should not leak information)
   - Log decryption failures without exposing data
   - Implement graceful degradation on key issues

4. **Testing**:
   - Unit tests for encryption/decryption
   - Integration tests for search functionality
   - Security tests for key management
   - Compliance tests (can be manual)

---

## Trade-offs and Limitations

### Acceptable Trade-offs

1. **Domain information exposure**:
    - **Public provider domains**: Trade: Domain names stored in plain text (e.g., "gmail.com")
      - Mitigation: Domains are public information; compliant with all jurisdictions
      - Justification: Enables efficient domain search; risk is minimal and acceptable
    - **Corporate domains**: Trade: Domain names stored in plain text (e.g., "company.com")
      - Mitigation: Treat as business-confidential with access controls and audit logging
      - Justification: Identifies organizations, not individuals; acceptable with proper controls
    - **Personal domains (.name, .me)**: Trade: Potentially PII stored in plain text
      - Mitigation: Document in privacy policy; local part encryption still protects complete email
      - Justification: Enables domain search; local part provides primary protection

2. **Pattern/substring search limitation**:
    - Trade: Cannot efficiently search for partial local parts (e.g., "john.*@gmail.com")
    - Mitigation: Implement in-memory filter with dataset size limits
    - Justification: Domain search + exact email match sufficient for most use cases

3. **Deterministic encryption vulnerability**:
    - Trade: Theoretical frequency analysis attack possible on encrypted local parts
    - Mitigation: Use 256-bit key; add user ID context if needed
    - Justification: Practical risk minimal for email addresses; complexity not justified

4. **Key rotation complexity**:
    - Trade: Key rotation requires data migration
    - Mitigation: Document process; implement in future if needed
    - Justification: Out of scope per requirements; acceptable given in-house preference

### Known Limitations

1. **Search functionality**: Exact email match + domain search supported; pattern/substring search limited (in-memory)
2. **Key management**: Manual process; no automated rotation
3. **Backup**: Requires custom implementation (no off-the-shelf solution)
4. **Information exposure**: Domain names visible in plain text (varies by domain type)
   - Public provider domains: 🟢 Acceptable (public information)
   - Corporate domains: 🟡 Acceptable with access controls (business data)
   - Personal domains (.name, .me): 🟡 Acceptable with documentation (PII but documented)
5. **Performance**: Pattern/substring searches slow on large datasets
6. **Personal domain profiling**: Potential to identify individuals from personal domains
   - Mitigation: Documentation, transparency, access controls, audit logging

### Compliance Gaps

**None identified**. Recommended approach meets or exceeds requirements for all analyzed jurisdictions:
- Public provider domains: Fully compliant (public information)
- Corporate domains: Compliant (organizational data, not individual PII)
- Personal domains: Compliant with transparency and proper documentation

---

## Conclusion

The recommended approach of **separated local/domain storage with deterministic AES-256-GCM encryption for local parts, environment variable key management, efficient domain search, and in-house file encryption for backups** provides:

- ✅ Strong security meeting all compliance requirements (EU, US, Canada, Brazil)
- ✅ Simple implementation without external dependencies
- ✅ Excellent performance for primary use cases (exact match + domain search)
- ✅ In-house control over all security components
- ✅ Ability to leverage existing project dependencies
- ✅ Domain search capability meeting user expectations
- ✅ Compliant storage across all domain types with proper documentation:
   - **Public provider domains**: Fully compliant, minimal security concerns
   - **Corporate domains**: Compliant with access controls and audit logging
   - **Personal domains (.name, .me)**: Compliant with transparency and documentation

The main trade-offs (limited pattern/substring search, domain exposure by type, manual key management) are acceptable given the project's constraints and preferences, with clear mitigation strategies documented.

**Implementation Priority**:
1. **Immediate**: Encrypt local parts, store domains as plain text (all types)
2. **Immediate**: Implement domain indexing and search functionality
3. **Immediate**: Document domain storage approach in privacy policy
4. **Short-term**: Implement database access controls and audit logging
5. **Future**: Consider domain classification for enhanced security if needed
6. **Future**: Consider encrypting personal domains if regulatory requirements evolve
- ✅ Ability to leverage existing project dependencies
- ✅ Domain search capability meeting user expectations
- ✅ Compliant domain storage (public information)

The main trade-offs (limited pattern/substring search, domain exposure, manual key management) are acceptable given the project's constraints and preferences.

**Next Steps**:
1. Create detailed implementation plan (this analysis)
2. Design database schema and migrations
3. Implement encryption service
4. Create email address CRUD operations
5. Implement search functionality
6. Add backup/restore utilities
7. Write comprehensive tests
8. Document security procedures

---

## References

### Compliance Documentation
- [GDPR Encryption Guidance](https://gdpr-info.eu/issues/encryption/)
- [UK ICO Encryption Guidance](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/security/encryption/)
- [PIPEDA Compliance Checklist](https://www.cookiebot.com/en/pipeda-compliance-checklist-and-requirements/)
- [LGPD Overview](https://termly.io/resources/articles/brazils-general-data-protection-law/)

### Encryption Best Practices
- [Deterministic Encryption - Google Tink](https://developers.google.com/tink/deterministic-encryption)
- [Field-Level Encryption](https://www.encryptionconsulting.com/field-level-encryption-ensuring-data-privacy-and-security/)
- [SQLite Encryption Options](https://www.sqliteforum.com/p/sqlite-encryption-and-secure-storage)

### Searchable Encryption
- [Searchable Symmetric Encryption - Wikipedia](https://en.wikipedia.org/wiki/Searchable_symmetric_encryption)
- [GEICO Searchable Encryption Blog](https://www.geico.com/techblog/searchable-field-level-encrypted-customer-pii/)
- [How to Search on Encrypted Data](https://esl.cs.brown.edu/blog/how-to-search-on-encrypted-data-introduction-part-1/)

### ZIP/Backup Security
- [Are password-protected ZIP files secure?](https://security.stackexchange.com/questions/35818/are-password-protected-zip-files-secure)
- [Does GDPR Require Encryption?](https://jetico.com/blog/does-gdpr-require-encryption/)

### Rust Cryptography
- [AES-GCM Crate Documentation](https://docs.rs/aes-gcm/latest/aes_gcm/)
- [Rust Crypto Project](https://github.com/RustCrypto)

---

**Document Version**: 1.0
**Last Updated**: 2026-01-02@19:37
