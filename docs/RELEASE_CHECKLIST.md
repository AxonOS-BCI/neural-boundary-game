# Release Checklist

```bash
bash scripts/smoke_check.sh
I_UNDERSTAND_REWRITE_HISTORY=YES bash scripts/force_clean_push_signed.sh
```

Then:

- enable GitHub Pages from Actions;
- verify 12 CI jobs;
- verify signed commit;
- create GitHub release from tag.
