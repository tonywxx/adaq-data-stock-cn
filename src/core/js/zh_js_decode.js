function d(t) {
            var e, n, i, o, r, a, s, l = (arguments,
            864e5), c = 7657, u = [], h = [], d = ~(3 << 30), f = 1 << 30, p = [0, 3, 5, 6, 9, 10, 12, 15, 17, 18, 20, 23, 24, 27, 29, 30], g = Math, v = function() {
                var l, c;
                for (l = 0; 64 > l; l++)
                    h[l] = g.pow(2, l),
                    26 > l && (u[l] = m(l + 65),
                    u[l + 26] = m(l + 97),
                    10 > l && (u[l + 52] = m(l + 48)));
                for (u.push("+", "/"),
                u = u.join(""),
                n = t.split(""),
                i = n.length,
                l = 0; i > l; l++)
                    n[l] = u.indexOf(n[l]);
                return o = {},
                e = a = 0,
                r = {},
                c = N([12, 6]),
                s = 63 ^ c[1],
                {
                    _1479: M,
                    _136: A,
                    _200: k,
                    _139: O,
                    _197: _mi_run,
                    _3466: _k2_run
                }["_" + c[0]] || function() {
                    return []
                }
            }, m = String.fromCharCode, b = function(t) {
                return t === {}._
            }, y = function() {
                var t, e;
                for (t = _(),
                e = 1; ; ) {
                    if (!_())
                        return e * (2 * t - 1);
                    e++
                }
            }, _ = function() {
                var t;
                return e >= i ? 0 : (t = n[e] & 1 << a,
                a++,
                a >= 6 && (a -= 6,
                e++),
                !!t)
            }, N = function(t, o, r) {
                var s, l, c, u, d;
                for (l = [],
                c = 0,
                o || (o = []),
                r || (r = []),
                s = 0; s < t.length; s++)
                    if (u = t[s],
                    c = 0,
                    u) {
                        if (e >= i)
                            return l;
                        if (t[s] <= 0)
                            c = 0;
                        else if (t[s] <= 30) {
                            for (; d = 6 - a,
                            d = u > d ? d : u,
                            c |= (n[e] >> a & (1 << d) - 1) << t[s] - u,
                            a += d,
                            a >= 6 && (a -= 6,
                            e++),
                            u -= d,
                            !(0 >= u); )
                                ;
                            o[s] && c >= h[t[s] - 1] && (c -= h[t[s]])
                        } else
                            c = N([30, t[s] - 30], [0, o[s]]),
                            r[s] || (c = c[0] + c[1] * h[30]);
                        l[s] = c
                    } else
                        l[s] = 0;
                return l
            }, x = function() {
                var t;
                return t = N([3])[0],
                1 == t ? (o.d = N([18], [1])[0],
                t = 0) : t || (t = N([6])[0]),
                t
            }, w = function(t) {
                var e, n, i;
                for (t > 1 && (e = 0),
                e = 0; t > e; e++)
                    o.d++,
                    i = o.d % 7,
                    (3 == i || 4 == i) && (o.d += 5 - i);
                return n = new Date,
                n.setTime((c + o.d) * l),
                n
            }, S = function(t) {
                var e, n, i;
                for (i = o.wd || 62,
                e = 0; t > e; e++)
                    do
                        o.d++;
                    while (!(i & 1 << (o.d % 7 + 10) % 7));
                return n = new Date,
                n.setTime((c + o.d) * l),
                n
            }, T = function(t) {
                var e, n, i;
                return t ? 0 > t ? (e = T(-t),
                [-e[0], -e[1]]) : (e = t % 3,
                n = (t - e) / 3,
                i = [n, n],
                e && i[e - 1]++,
                i) : [0, 0]
            }, C = function(t, e, n) {
                var i, o, r;
                for (o = "number" == typeof e ? T(e) : e,
                r = T(n),
                i = [r[0] - o[0], r[1] - o[1]],
                o = 1; i[0] < i[1]; )
                    o *= 5,
                    i[1]--;
                for (; i[1] < i[0]; )
                    o *= 2,
                    i[0]--;
                if (o > 1 && (t *= o),
                i = i[0],
                t = _decnum(t),
                0 > i) {
                    for (; t.length + i <= 0; )
                        t = "0" + t;
                    return i += t.length,
                    o = t.substr(0, i) - 0,
                    void 0 === n ? o + "." + t.substr(i) - 0 : (r = t.charAt(i) - 0,
                    r > 5 ? o++ : 5 == r && (t.substr(i + 1) - 0 > 0 ? o++ : o += 1 & o),
                    o)
                }
                for (; i > 0; i--)
                    t += "0";
                return t - 0
            }, k = function() {
                var t, n, r, a, l;
                if (s >= 1)
                    return [];
                for (o.d = N([18], [1])[0] - 1,
                r = N([3, 3, 30, 6]),
                o.p = r[0],
                o.ld = r[1],
                o.cd = r[2],
                o.c = r[3],
                o.m = g.pow(10, o.p),
                o.pc = o.cd / o.m,
                n = [],
                t = 0; a = {
                    d: 1
                },
                _() && (r = N([3])[0],
                0 == r ? a.d = N([6])[0] : 1 == r ? (o.d = N([18])[0],
                a.d = 0) : a.d = r),
                l = {
                    date: w(a.d)
                },
                _() && (o.ld += y()),
                r = N([3 * o.ld], [1]),
                o.cd += r[0],
                l.close = o.cd / o.m,
                n.push(l),
                !(e >= i) && (e != i - 1 || 63 & (o.c ^ t + 1)); t++)
                    ;
                return n[0].prevclose = o.pc,
                n
            }, A = function() {
                var t, n, r, a, l, c, u, h, d, f, p;
                if (s > 2)
                    return [];
                for (u = [],
                d = {
                    v: "volume",
                    p: "price",
                    a: "avg_price"
                },
                o.d = N([18], [1])[0] - 1,
                h = {
                    date: w(1)
                },
                r = N(1 > s ? [3, 3, 4, 1, 1, 1, 5] : [4, 4, 4, 1, 1, 1, 3]),
                t = 0; 7 > t; t++)
                    o[["la", "lp", "lv", "tv", "rv", "zv", "pp"][t]] = r[t];
                for (o.m = g.pow(10, o.pp),
                s >= 1 ? (r = N([3, 3]),
                o.c = r[0],
                r = r[1]) : (r = 5,
                o.c = 2),
                o.pc = N([6 * r])[0],
                h.pc = o.pc / o.m,
                o.cp = o.pc,
                o.da = 0,
                o.sa = o.sv = 0,
                t = 0; !(e >= i) && (e != i - 1 || 7 & (o.c ^ t)); t++) {
                    for (l = {},
                    a = {},
                    f = o.tv ? _() : 1,
                    n = 0; 3 > n; n++)
                        if (p = ["v", "p", "a"][n],
                        (f ? _() : 0) && (r = y(),
                        o["l" + p] += r),
                        c = "v" == p && o.rv ? _() : 1,
                        r = N([3 * o["l" + p] + ("v" == p ? 7 * c : 0)], [!!n])[0] * (c ? 1 : 100),
                        a[p] = r,
                        "v" == p) {
                            if (!(l[d[p]] = r) && (s > 1 || 241 > t) && (o.zv ? !_() : 1)) {
                                a.p = 0;
                                break
                            }
                        } else
                            "a" == p && (o.da = (1 > s ? 0 : o.da) + a.a);
                    o.sv += a.v,
                    l[d.p] = (o.cp += a.p) / o.m,
                    o.sa += a.v * o.cp,
                    l[d.a] = b(a.a) ? t ? u[t - 1][d.a] : l[d.p] : o.sv ? ((g.floor((o.sa * (2e3 / o.m) + o.sv) / o.sv) >> 1) + o.da) / 1e3 : l[d.p] + o.da / 1e3,
                    u.push(l)
                }
                return u[0].date = h.date,
                u[0].prevclose = h.pc,
                u
            }, M = function() {
                var t, e, n, i, r, a, l;
                if (s >= 1)
                    return [];
                for (o.lv = 0,
                o.ld = 0,
                o.cd = 0,
                o.cv = [0, 0],
                o.p = N([6])[0],
                o.d = N([18], [1])[0] - 1,
                o.m = g.pow(10, o.p),
                r = N([3, 3]),
                o.md = r[0],
                o.mv = r[1],
                t = []; r = N([6]),
                r.length; ) {
                    if (n = {
                        c: r[0]
                    },
                    i = {},
                    n.d = 1,
                    32 & n.c)
                        for (; ; ) {
                            if (r = N([6])[0],
                            63 == (16 | r)) {
                                l = 16 & r ? "x" : "u",
                                r = N([3, 3]),
                                n[l + "_d"] = r[0] + o.md,
                                n[l + "_v"] = r[1] + o.mv;
                                break
                            }
                            if (32 & r) {
                                a = 8 & r ? "d" : "v",
                                l = 16 & r ? "x" : "u",
                                n[l + "_" + a] = (7 & r) + o["m" + a];
                                break
                            }
                            if (a = 15 & r,
                            0 == a ? n.d = N([6])[0] : 1 == a ? (o.d = a = N([18])[0],
                            n.d = 0) : n.d = a,
                            !(16 & r))
                                break
                        }
                    i.date = w(n.d);
                    for (a in {
                        v: 0,
                        d: 0
                    })
                        b(n["x_" + a]) || (o["l" + a] = n["x_" + a]),
                        b(n["u_" + a]) && (n["u_" + a] = o["l" + a]);
                    for (n.l_l = [n.u_d, n.u_d, n.u_d, n.u_d, n.u_v],
                    l = p[15 & n.c],
                    1 & n.u_v && (l = 31 - l),
                    16 & n.c && (n.l_l[4] += 2),
                    e = 0; 5 > e; e++)
                        l & 1 << 4 - e && n.l_l[e]++,
                        n.l_l[e] *= 3;
                    n.d_v = N(n.l_l, [1, 0, 0, 1, 1], [0, 0, 0, 0, 1]),
                    a = o.cd + n.d_v[0],
                    i.open = a / o.m,
                    i.high = (a + n.d_v[1]) / o.m,
                    i.low = (a - n.d_v[2]) / o.m,
                    i.close = (a + n.d_v[3]) / o.m,
                    r = n.d_v[4],
                    "number" == typeof r && (r = [r, r >= 0 ? 0 : -1]),
                    o.cd = a + n.d_v[3],
                    l = o.cv[0] + r[0],
                    o.cv = [l & d, o.cv[1] + r[1] + !!((o.cv[0] & d) + (r[0] & d) & f)],
                    i.volume = (o.cv[0] & f - 1) + o.cv[1] * f,
                    t.push(i)
                }
                return t
            }, O = function() {
                var t, e, n, i;
                if (s > 1)
                    return [];
                for (o.l = 0,
                i = -1,
                o.d = N([18])[0] - 1,
                n = N([18])[0]; o.d < n; )
                    e = w(1),
                    0 >= i ? (_() && (o.l += y()),
                    i = N([3 * o.l], [0])[0] + 1,
                    t || (t = [e],
                    i--)) : t.push(e),
                    i--;
                return t
            };
            return _mi_run = function() {
                var t, n, r, a;
                if (s >= 1)
                    return [];
                for (o.f = N([6])[0],
                o.c = N([6])[0],
                r = [],
                o.dv = [],
                o.dl = [],
                t = 0; t < o.f; t++)
                    o.dv[t] = 0,
                    o.dl[t] = 0;
                for (t = 0; !(e >= i) && (e != i - 1 || 7 & (o.c ^ t)); t++) {
                    for (a = [],
                    n = 0; n < o.f; n++)
                        _() && (o.dl[n] += y()),
                        o.dv[n] += N([3 * o.dl[n]], [1])[0],
                        a[n] = o.dv[n];
                    r.push(a)
                }
                return r
            }
            ,
            _k2_run = function() {
                if (o = {
                    b_avp: 1,
                    b_ph: 0,
                    b_phx: 0,
                    b_sep: 0,
                    p_p: 6,
                    p_v: 0,
                    p_a: 0,
                    p_e: 0,
                    p_t: 0,
                    l_o: 3,
                    l_h: 3,
                    l_l: 3,
                    l_c: 3,
                    l_v: 5,
                    l_a: 5,
                    l_e: 3,
                    l_t: 0,
                    u_p: 0,
                    u_v: 0,
                    u_a: 0,
                    wd: 62,
                    d: 0
                },
                s > 0)
                    return [];
                var t, n, r, a, l, c, u;
                for (t = []; ; ) {
                    if (e >= i)
                        return void 0;
                    if (r = {
                        d: 1,
                        c: 0
                    },
                    _())
                        if (_()) {
                            if (_()) {
                                for (r.c++,
                                r.a = o.b_avp,
                                _() && (o.b_avp ^= _(),
                                o.b_ph ^= _(),
                                o.b_phx ^= _(),
                                r.s = o.b_sep,
                                o.b_sep ^= _(),
                                _() && (o.wd = N([7])[0]),
                                r.s ^ o.b_sep && (r.s ? o.u_p = o.u_c : o.u_o = o.u_h = o.u_l = o.u_c = o.u_p)),
                                c = 0; c < 3 + 2 * o.b_ph; c++)
                                    if (_() && (l = "pvaet".charAt(c),
                                    a = o["p_" + l],
                                    o["p_" + l] += y(),
                                    o["u_" + l] = C(o["u_" + l], a, o["p_" + l]) - 0,
                                    o.b_sep && !c))
                                        for (u = 0; 4 > u; u++)
                                            l = "ohlc".charAt(u),
                                            o["u_" + l] = C(o["u_" + l], a, o.p_p) - 0;
                                !o.b_avp && r.a && (o.u_a = C(n && n.amount || 0, 0, o.p_a))
                            }
                            if (_())
                                for (r.c++,
                                c = 0; c < 7 + o.b_ph + o.b_phx; c++)
                                    _() && (6 == c ? r.d = x() : o["l_" + "ohlcva*et".charAt(c)] += y());
                            if (_() && (r.c++,
                            l = o.l_o + (_() && y()),
                            a = N([3 * l], [1])[0],
                            r.p = o.b_sep ? o.u_c + a : o.u_p += a),
                            !r.c)
                                break
                        } else
                            _() ? _() ? _() ? r.d = x() : o.l_v += y() : o.b_ph && _() ? o["l_" + "et".charAt(o.b_phx && _())] += y() : o.l_a += y() : o["l_" + "ohlc".charAt(N([2])[0])] += y();
                    for (c = 0; c < 6 + o.b_ph + o.b_phx; c++)
                        u = "ohlcvaet".charAt(c),
                        a = (o.b_sep ? 191 : 185) >> c & 1,
                        r["v_" + u] = N([3 * o["l_" + u]], [a])[0];
                    n = {
                        date: S(r.d)
                    },
                    r.p && (n.prevclose = C(r.p, o.p_p)),
                    o.b_sep ? (n.open = C(o.u_o += r.v_o, o.p_p),
                    n.high = C(o.u_h += r.v_h, o.p_p),
                    n.low = C(o.u_l += r.v_l, o.p_p),
                    n.close = C(o.u_c += r.v_c, o.p_p)) : (r.o = o.u_p + r.v_o,
                    n.open = C(r.o, o.p_p),
                    n.high = C(r.o + r.v_h, o.p_p),
                    n.low = C(r.o - r.v_l, o.p_p),
                    n.close = C(o.u_p = r.o + r.v_c, o.p_p)),
                    n.volume = C(o.u_v += r.v_v, o.p_v),
                    o.b_avp ? (a = T(o.p_p),
                    l = T(o.p_v),
                    n.amount = C(C(Math.floor((o.b_sep ? (o.u_o + o.u_h + o.u_l + o.u_c) / 4 : r.o + (r.v_h - r.v_l + r.v_c) / 4) * o.u_v + .5), [a[0] + l[0], a[1] + l[1]], o.p_a) + r.v_a, o.p_a)) : (o.u_a += r.v_a,
                    n.amount = C(o.u_a, o.p_a)),
                    o.b_ph && (n.postVol = C(r.v_e, o.p_e),
                    n.postAmt = C(Math.floor(n.postVol * n.close + (o.b_phx ? C(r.v_t, o.p_t) : 0) + .5), 0)),
                    t.push(n)
                }
                return t
            }
            ,
            _decnum = function(t) {
                var e, n, i;
                if (t = (t || 0).toString(),
                i = [],
                n = t.toLowerCase().indexOf("e"),
                n > 0) {
                    for (e = t.substr(n + 1) - 0; e >= 0; e--)
                        i.push(Math.floor(e * Math.pow(10, -e) + .5) - 0);
                    return i.join("")
                }
                return t
            }
            ,
            v()()
        }
        ;
