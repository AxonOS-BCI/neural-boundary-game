# Release Checklist

```bash
bash scripts/smoke_check.sh
I_UNDERSTAND_REWRITE_HISTORY=YES bash scripts/force_clean_push_signed.sh
```

Then verify:

- 12 CI jobs are green;
- Pages static-site check is green;
- signed tag is Verified;
- GitHub Release workflow has created release assets;
- README release badge is green;
- README tag badge points to the latest tag.
