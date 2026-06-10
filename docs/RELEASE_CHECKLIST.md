# Release Checklist

```bash
bash scripts/smoke_check.sh
I_UNDERSTAND_REWRITE_HISTORY=YES bash scripts/force_clean_push_signed.sh
```

Then verify:

- 12 CI jobs are green;
- Pages deployment is green;
- signed tag is Verified;
- GitHub Release was created by the Release workflow;
- README release badge is green;
- README tag badge points to the latest tag.
