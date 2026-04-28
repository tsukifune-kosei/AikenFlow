# Generated Adversarial Cases

## Deposit

- `missing_signature`: Omit required signer `$owner`.
- `non_positive_value`: Use zero or negative value for `$amount`.

## Withdraw

- `missing_signature`: Omit required signer `datum.owner`.
- `premature_validity_interval`: Set validity interval before `datum.deadline`.

## EmergencyCancel

- `missing_signature`: Omit required signer `datum.owner`.
- `expired_validity_interval`: Set validity interval after `datum.deadline`.
