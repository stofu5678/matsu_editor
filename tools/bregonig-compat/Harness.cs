/*
 * Runs the real bregonig.dll (the Unicode API) the way Sakura Editor calls it, and writes what it
 * does to a JSON file.
 *
 * This file was written for MATSU Editor. The BREGEXP_W structure, the entry points and the call
 * sequence (compile with BMatchW / BSubstW on a dummy text, then BMatchExW / BSubstExW with no
 * pattern) come from Sakura Editor's sakura_core/extmodule/CBregexpDll2.h, CBregexpDll2.cpp and
 * CBregexp.cpp. The pattern text is built as CBregexp::MakePatternSub builds it. Modified: ported
 * from C++ to C#, reduced to what a measurement needs.
 *
 * Original files:
 *   Copyright (C) 2001-2002, genta
 *   Copyright (C) 2002, novice, hor, Azumaiya
 *   Copyright (C) 2003, Karoto
 *   Copyright (C) 2005, Karoto
 *   Copyright (C) 2006, Karoto
 *   Copyright (C) 2007, ryoji
 *   Copyright (C) 2008, kobake
 *   Copyright (C) 2018-2022, Sakura Editor Organization
 *
 * zlib License
 *
 * This software is provided 'as-is', without any express or implied
 * warranty.  In no event will the authors be held liable for any damages
 * arising from the use of this software.
 *
 * Permission is granted to anyone to use this software for any purpose,
 * including commercial applications, and to alter it and redistribute it
 * freely, subject to the following restrictions:
 *
 * 1. The origin of this software must not be misrepresented; you must not
 *    claim that you wrote the original software. If you use this software
 *    in a product, an acknowledgment in the product documentation would be
 *    appreciated but is not required.
 * 2. Altered source versions must be plainly marked as such, and must not be
 *    misrepresented as being the original software.
 * 3. This notice may not be removed or altered from any source distribution.
 *
 * Usage:
 *   Harness.exe cases.json result.json   run every case, each in its own process
 *   Harness.exe --one cases.json N out   run case N (used by the line above)
 *
 * Built with the C# 5 compiler that ships with Windows, so no newer syntax is used.
 */
using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Web.Script.Serialization;

[StructLayout(LayoutKind.Sequential)]
struct BregexpW
{
    public IntPtr outp;
    public IntPtr outendp;
    public int splitctr;
    public IntPtr splitp;
    public int rsv1;
    public IntPtr parap;
    public IntPtr paraendp;
    public IntPtr transtblp;
    public IntPtr startp;
    public IntPtr endp;
    public int nparens;
}

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
delegate int MatchW(IntPtr str, IntPtr target, IntPtr targetendp, ref IntPtr rxp, IntPtr msg);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
delegate int MatchExW(IntPtr str, IntPtr targetbeg, IntPtr target, IntPtr targetendp, ref IntPtr rxp, IntPtr msg);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
delegate void RegfreeW(IntPtr rx);

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
delegate IntPtr VersionW();

static class Native
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryW(string name);

    [DllImport("kernel32.dll", CharSet = CharSet.Ansi, SetLastError = true)]
    public static extern IntPtr GetProcAddress(IntPtr module, string name);
}

class Dll
{
    public MatchW BMatch;
    public MatchW BSubst;
    public MatchExW BMatchEx;
    public MatchExW BSubstEx;
    public RegfreeW BRegfree;
    public VersionW BRegexpVersion;

    static T Get<T>(IntPtr module, string name) where T : class
    {
        IntPtr p = Native.GetProcAddress(module, name);
        if (p == IntPtr.Zero)
            throw new Exception("missing export " + name);
        return (T)(object)Marshal.GetDelegateForFunctionPointer(p, typeof(T));
    }

    public static Dll Load(string path)
    {
        IntPtr module = Native.LoadLibraryW(path);
        if (module == IntPtr.Zero)
            throw new Exception("cannot load " + path + " (error " + Marshal.GetLastWin32Error() + ")");
        Dll d = new Dll();
        d.BMatch = Get<MatchW>(module, "BMatchW");
        d.BSubst = Get<MatchW>(module, "BSubstW");
        d.BMatchEx = Get<MatchExW>(module, "BMatchExW");
        d.BSubstEx = Get<MatchExW>(module, "BSubstExW");
        d.BRegfree = Get<RegfreeW>(module, "BRegfreeW");
        d.BRegexpVersion = Get<VersionW>(module, "BRegexpVersionW");
        return d;
    }
}

static class Json
{
    // False when the text has a surrogate that has no partner; JSON text cannot carry that
    // faithfully into most readers, so such a text is written as a list of UTF-16 units instead.
    public static bool WellFormed(string s)
    {
        for (int i = 0; i < s.Length; i++)
        {
            if (char.IsHighSurrogate(s[i]))
            {
                if (i + 1 >= s.Length || !char.IsLowSurrogate(s[i + 1])) return false;
                i++;
            }
            else if (char.IsLowSurrogate(s[i]))
                return false;
        }
        return true;
    }

    public static string Units(string s)
    {
        StringBuilder b = new StringBuilder("[");
        for (int i = 0; i < s.Length; i++)
            b.Append(i > 0 ? "," : "").Append((int)s[i]);
        return b.Append(']').ToString();
    }

    // Only ASCII is written, so the file does not depend on an encoding.
    public static string Quote(string s)
    {
        if (s == null)
            return "null";
        StringBuilder b = new StringBuilder("\"");
        foreach (char c in s)
        {
            if (c == '"') b.Append("\\\"");
            else if (c == '\\') b.Append("\\\\");
            else if (c >= 0x20 && c < 0x7f) b.Append(c);
            else b.Append("\\u").Append(((int)c).ToString("x4"));
        }
        return b.Append('"').ToString();
    }
}

class Program
{
    // Sakura's delimiter (WCODE::BREGEXP_DELIMITER in sakura_core/charset/charcode.h).
    const char Delimiter = '￿';

    static IntPtr AllocWide(string s, int extra)
    {
        IntPtr p = Marshal.AllocHGlobal((s.Length + extra) * 2);
        for (int i = 0; i < s.Length; i++)
            Marshal.WriteInt16(p, i * 2, (short)s[i]);
        for (int i = 0; i < extra; i++)
            Marshal.WriteInt16(p, (s.Length + i) * 2, 0);
        return p;
    }

    static string ReadWide(IntPtr p, int len)
    {
        char[] chars = new char[len];
        for (int i = 0; i < len; i++)
            chars[i] = (char)Marshal.ReadInt16(p, i * 2);
        return new string(chars);
    }

    // The message buffer holds at most 80 characters (BREGEXP_MAX_ERROR_MESSAGE_LEN).
    static string ReadMsg(IntPtr msg)
    {
        string s = ReadWide(msg, 80);
        int nul = s.IndexOf('\0');
        return nul < 0 ? s : s.Substring(0, nul);
    }

    // CBregexp::MakePatternAlternate: `.` becomes `[^\r\n]` and `$` becomes `(?<![\r\n])(?=\r|$)`,
    // except in a character class, after a backslash and between \Q and \E. Sakura's
    // CBregexp::Compile always does this before it builds the pattern for the DLL.
    // States: 0 DEF, 1 D_E, 2 D_C, 3 CHA, 4 C_E, 5 C_C, 6 QEE, 7 Q_E.
    // Classes: 0 OTHER, 1 DOT, 2 DOLLAR, 3 c, 4 Q, 5 E, 6 [, 7 ], 8 backslash.
    // Special results: -1 enter class, -2 exit class, -3 dot, -4 dollar.
    static readonly int[,] Table = new int[,] {
        /* DEF */ { 0, -3, -4, 0, 0, 0, -1, 0, 1 },
        /* D_E */ { 0, 0, 0, 2, 6, 0, 0, 0, 0 },
        /* D_C */ { 0, 0, 0, 0, 0, 0, 0, 0, 1 },
        /* CHA */ { 3, 3, 3, 3, 3, 3, -1, -2, 4 },
        /* C_E */ { 3, 3, 3, 5, 3, 3, 3, 3, 3 },
        /* C_C */ { 3, 3, 3, 3, 3, 3, 3, 3, 4 },
        /* QEE */ { 6, 6, 6, 6, 6, 6, 6, 6, 7 },
        /* Q_E */ { 6, 6, 6, 6, 6, 0, 6, 6, 7 }
    };

    static string MakePatternAlternate(string pattern)
    {
        StringBuilder b = new StringBuilder();
        int state = 0;
        int level = 0;
        foreach (char ch in pattern)
        {
            int cls = ch == '.' ? 1 : ch == '$' ? 2 : ch == 'c' ? 3 : ch == 'Q' ? 4 : ch == 'E' ? 5
                : ch == '[' ? 6 : ch == ']' ? 7 : ch == '\\' ? 8 : 0;
            int next = Table[state, cls];
            if (next >= 0)
            {
                state = next;
                b.Append(ch);
            }
            else if (next == -1)
            {
                level++;
                state = 3;
                b.Append(ch);
            }
            else if (next == -2)
            {
                level--;
                state = level > 0 ? 3 : 0;
                b.Append(ch);
            }
            else if (next == -3)
                b.Append("[^\\r\\n]");
            else
                b.Append("(?<![\\r\\n])(?=\\r|$)");
        }
        return b.ToString();
    }

    // CBregexp::MakePatternSub.
    static string MakePattern(string kind, string pattern, string replacement, bool matchCase, string extra)
    {
        StringBuilder b = new StringBuilder();
        bool subst = kind != "match";
        b.Append(subst ? 's' : 'm');
        b.Append(Delimiter).Append(MakePatternAlternate(pattern)).Append(Delimiter);
        if (subst)
            b.Append(replacement).Append(Delimiter);
        b.Append("km");
        if (!matchCase) b.Append('i');
        if (kind == "replace_all") b.Append('g');
        b.Append(extra);
        return b.ToString();
    }

    static string Str(Dictionary<string, object> c, string key, string dflt)
    {
        object v;
        return c.TryGetValue(key, out v) && v != null ? (string)v : dflt;
    }

    static int Int(Dictionary<string, object> c, string key, int dflt)
    {
        object v;
        return c.TryGetValue(key, out v) && v != null ? Convert.ToInt32(v) : dflt;
    }

    static bool Bool(Dictionary<string, object> c, string key, bool dflt)
    {
        object v;
        return c.TryGetValue(key, out v) && v != null ? (bool)v : dflt;
    }

    static string RunCase(Dll dll, Dictionary<string, object> c)
    {
        string id = Str(c, "id", "");
        string kind = Str(c, "kind", "match");
        string pattern = Str(c, "pattern", "");
        string replacement = Str(c, "replacement", "");
        string subject = Str(c, "subject", "");
        int start = Int(c, "start", 0);
        bool matchCase = Bool(c, "match_case", false);
        string extra = Str(c, "extra_options", "");

        string whole = MakePattern(kind, pattern, replacement, matchCase, extra);
        IntPtr pat = AllocWide(whole, 1);
        IntPtr dummy = AllocWide("", 2);
        IntPtr subj = AllocWide(subject, 1);
        IntPtr msg = AllocWide("", 255);
        IntPtr rx = IntPtr.Zero;
        int ret;
        bool subst = kind != "match";

        // CBregexp::Compile: the first call compiles against a dummy text.
        MatchW first = subst ? dll.BSubst : dll.BMatch;
        int compileRet = first(pat, dummy, (IntPtr)(dummy.ToInt64() + 2), ref rx, msg);
        string compileMsg = ReadMsg(msg);

        StringBuilder o = new StringBuilder();
        o.Append("{\"id\":").Append(Json.Quote(id));
        o.Append(",\"compile_ret\":").Append(compileRet);
        o.Append(",\"compile_msg\":").Append(Json.Quote(compileMsg));

        if (compileMsg.Length > 0 || rx == IntPtr.Zero)
        {
            o.Append(",\"ran\":false}");
            if (rx != IntPtr.Zero) dll.BRegfree(rx);
            return o.ToString();
        }

        // CBregexp::Match / Replace: a second call with no pattern, on the real text.
        Marshal.WriteInt16(msg, 0, 0);
        long b0 = subj.ToInt64();
        IntPtr tbeg = subj;
        IntPtr tcur = (IntPtr)(b0 + 2L * start);
        IntPtr tend = (IntPtr)(b0 + 2L * Int(c, "end", subject.Length));
        ret = (subst ? dll.BSubstEx : dll.BMatchEx)(IntPtr.Zero, tbeg, tcur, tend, ref rx, msg);
        string runMsg = ReadMsg(msg);

        o.Append(",\"ran\":true,\"ret\":").Append(ret);
        o.Append(",\"msg\":").Append(Json.Quote(runMsg));

        BregexpW r = (BregexpW)Marshal.PtrToStructure(rx, typeof(BregexpW));
        o.Append(",\"nparens\":").Append(r.nparens);

        if (ret > 0 && r.startp != IntPtr.Zero && r.endp != IntPtr.Zero)
        {
            o.Append(",\"groups\":[");
            for (int i = 0; i <= r.nparens; i++)
            {
                IntPtr s = Marshal.ReadIntPtr(r.startp, i * IntPtr.Size);
                IntPtr e = Marshal.ReadIntPtr(r.endp, i * IntPtr.Size);
                if (i > 0) o.Append(',');
                if (s == IntPtr.Zero || e == IntPtr.Zero)
                    o.Append("null");
                else
                    o.Append('[').Append((s.ToInt64() - b0) / 2).Append(',').Append((e.ToInt64() - b0) / 2).Append(']');
            }
            o.Append(']');
        }

        if (subst)
        {
            if (r.outp == IntPtr.Zero)
                o.Append(",\"out\":null");
            else
            {
                long n = (r.outendp.ToInt64() - r.outp.ToInt64()) / 2;
                string text = ReadWide(r.outp, (int)n);
                if (Json.WellFormed(text))
                    o.Append(",\"out\":").Append(Json.Quote(text));
                else
                    o.Append(",\"out_units\":").Append(Json.Units(text));
            }
        }
        o.Append('}');
        dll.BRegfree(rx);
        return o.ToString();
    }

    static List<Dictionary<string, object>> LoadCases(string path)
    {
        string text = File.ReadAllText(path, new UTF8Encoding(false));
        JavaScriptSerializer js = new JavaScriptSerializer();
        js.MaxJsonLength = int.MaxValue;
        Dictionary<string, object> root = js.Deserialize<Dictionary<string, object>>(text);
        List<Dictionary<string, object>> list = new List<Dictionary<string, object>>();
        foreach (object item in (ArrayList)root["cases"])
            list.Add((Dictionary<string, object>)item);
        return list;
    }

    static int Main(string[] args)
    {
        string dir = AppDomain.CurrentDomain.BaseDirectory;
        if (args.Length == 4 && args[0] == "--one")
        {
            List<Dictionary<string, object>> cases = LoadCases(args[1]);
            Dll dll = Dll.Load(Path.Combine(dir, "bregonig.dll"));
            string json = RunCase(dll, cases[int.Parse(args[2])]);
            File.WriteAllText(args[3], json, new UTF8Encoding(false));
            return 0;
        }
        if (args.Length != 2)
        {
            Console.Error.WriteLine("usage: Harness cases.json result.json");
            return 2;
        }
        List<Dictionary<string, object>> all = LoadCases(args[0]);
        Dll d = Dll.Load(Path.Combine(dir, "bregonig.dll"));
        string version = ReadWide(d.BRegexpVersion(), 64);
        int nul = version.IndexOf('\0');
        if (nul >= 0) version = version.Substring(0, nul);

        StringBuilder outp = new StringBuilder();
        outp.Append("{\"dll_version\":").Append(Json.Quote(version));
        outp.Append(",\"pointer_size\":").Append(IntPtr.Size);
        outp.Append(",\"results\":[\n");
        string tmp = Path.Combine(Path.GetTempPath(), "bregonig-one-" + Process.GetCurrentProcess().Id + ".json");
        for (int i = 0; i < all.Count; i++)
        {
            string id = Str(all[i], "id", "");
            if (File.Exists(tmp)) File.Delete(tmp);
            ProcessStartInfo psi = new ProcessStartInfo(Path.Combine(dir, "Harness.exe"),
                "--one \"" + args[0] + "\" " + i + " \"" + tmp + "\"");
            psi.UseShellExecute = false;
            Process p = Process.Start(psi);
            p.WaitForExit();
            string line;
            if (p.ExitCode == 0 && File.Exists(tmp))
                line = File.ReadAllText(tmp, new UTF8Encoding(false));
            else
                line = "{\"id\":" + Json.Quote(id) + ",\"crashed\":true,\"exit_code\":" + p.ExitCode + "}";
            outp.Append(i > 0 ? ",\n" : "").Append(line);
        }
        outp.Append("\n]}\n");
        if (File.Exists(tmp)) File.Delete(tmp);
        File.WriteAllText(args[1], outp.ToString(), new UTF8Encoding(false));
        return 0;
    }
}
