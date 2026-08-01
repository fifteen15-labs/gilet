import pickle, os, statistics, collections

S = os.path.dirname(os.path.abspath(__file__))
d = open(os.path.join(S, "frames/f0003.bin"), "rb").read()
recs = pickle.load(open(os.path.join(S, "recs.pkl"), "rb"))

W = 160  # bytes after name to inspect
cols = collections.defaultdict(list)
for start, nameid, name, nend in recs:
    tail = d[nend : nend + W]
    if len(tail) < W:
        continue
    for i, b in enumerate(tail):
        cols[i].append(b)

n = len(recs)
print(f"records {n:,}, inspecting {W} byte-columns after name\n")

# A CA/PA column: values mostly in 1..200, wide spread, not mostly 0/255
cands = []
for i in range(W):
    v = cols[i]
    inrange = sum(1 for x in v if 1 <= x <= 200) / len(v)
    zeros = sum(1 for x in v if x == 0) / len(v)
    uniq = len(set(v))
    if inrange > 0.95 and zeros < 0.10 and uniq > 60:
        cands.append(i)
        mean = statistics.mean(v)
        print(f"  col +{i:>3}  inrange={inrange:.2f} uniq={uniq:>3} mean={mean:6.1f} "
              f"min={min(v):>3} max={max(v):>3}")

print(f"\ncandidate columns: {cands}\n")

# PA >= CA constraint across all pairs
print("pairs satisfying value[j] >= value[i] for >99% of players (CA=i, PA=j):")
best = []
for i in cands:
    for j in cands:
        if i == j:
            continue
        a, b = cols[i], cols[j]
        ok = sum(1 for x, y in zip(a, b) if y >= x) / len(a)
        if ok > 0.99:
            gap = statistics.mean(y - x for x, y in zip(a, b))
            best.append((ok, i, j, gap))
for ok, i, j, gap in sorted(best, key=lambda t: -t[0])[:20]:
    print(f"  CA=+{i:<3} PA=+{j:<3}  ok={ok:.4f}  mean_gap={gap:5.1f}  "
          f"CAmean={statistics.mean(cols[i]):.1f} PAmean={statistics.mean(cols[j]):.1f}")
