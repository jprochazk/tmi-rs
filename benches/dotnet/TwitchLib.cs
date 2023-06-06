using System.Collections.ObjectModel;
using System.Drawing;
using System.Text;
using System.Text.RegularExpressions;
using Utilities;

namespace forsen;
internal class TwitchLib
{
    private static MessageEmoteCollection _channelEmotes = new MessageEmoteCollection();

    public static IrcMessage ParseIrcMessage(string raw)
    {
        Dictionary<string, string> tagDict = new Dictionary<string, string>();

        ParserState state = ParserState.STATE_NONE;
        int[] starts = new[] { 0, 0, 0, 0, 0, 0 };
        int[] lens = new[] { 0, 0, 0, 0, 0, 0 };
        for (int i = 0; i < raw.Length; ++i)
        {
            lens[(int)state] = i - starts[(int)state] - 1;
            if (state == ParserState.STATE_NONE && raw[i] == '@')
            {
                state = ParserState.STATE_V3;
                starts[(int)state] = ++i;

                int start = i;
                string key = null;
                for (; i < raw.Length; ++i)
                {
                    if (raw[i] == '=')
                    {
                        key = raw.Substring(start, i - start);
                        start = i + 1;
                    }
                    else if (raw[i] == ';')
                    {
                        if (key == null)
                            tagDict[raw.Substring(start, i - start)] = "1";
                        else
                            tagDict[key] = raw.Substring(start, i - start);
                        start = i + 1;
                    }
                    else if (raw[i] == ' ')
                    {
                        if (key == null)
                            tagDict[raw.Substring(start, i - start)] = "1";
                        else
                            tagDict[key] = raw.Substring(start, i - start);
                        break;
                    }
                }
            }
            else if (state < ParserState.STATE_PREFIX && raw[i] == ':')
            {
                state = ParserState.STATE_PREFIX;
                starts[(int)state] = ++i;
            }
            else if (state < ParserState.STATE_COMMAND)
            {
                state = ParserState.STATE_COMMAND;
                starts[(int)state] = i;
            }
            else if (state < ParserState.STATE_TRAILING && raw[i] == ':')
            {
                state = ParserState.STATE_TRAILING;
                starts[(int)state] = ++i;
                break;
            }
            else if (state < ParserState.STATE_TRAILING && raw[i] == '+' || state < ParserState.STATE_TRAILING && raw[i] == '-')
            {
                state = ParserState.STATE_TRAILING;
                starts[(int)state] = i;
                break;
            }
            else if (state == ParserState.STATE_COMMAND)
            {
                state = ParserState.STATE_PARAM;
                starts[(int)state] = i;
            }

            while (i < raw.Length && raw[i] != ' ')
                ++i;
        }

        lens[(int)state] = raw.Length - starts[(int)state];
        string cmd = raw.Substring(starts[(int)ParserState.STATE_COMMAND],
            lens[(int)ParserState.STATE_COMMAND]);

        IrcCommand2 command = IrcCommand2.Unknown;
        switch (cmd)
        {
            case "PRIVMSG":
                command = IrcCommand2.PrivMsg;
                break;
            case "NOTICE":
                command = IrcCommand2.Notice;
                break;
            case "PING":
                command = IrcCommand2.Ping;
                break;
            case "PONG":
                command = IrcCommand2.Pong;
                break;
            case "CLEARCHAT":
                command = IrcCommand2.ClearChat;
                break;
            case "CLEARMSG":
                command = IrcCommand2.ClearMsg;
                break;
            case "USERSTATE":
                command = IrcCommand2.UserState;
                break;
            case "GLOBALUSERSTATE":
                command = IrcCommand2.GlobalUserState;
                break;
            case "NICK":
                command = IrcCommand2.Nick;
                break;
            case "JOIN":
                command = IrcCommand2.Join;
                break;
            case "PART":
                command = IrcCommand2.Part;
                break;
            case "PASS":
                command = IrcCommand2.Pass;
                break;
            case "CAP":
                command = IrcCommand2.Cap;
                break;
            case "001":
                command = IrcCommand2.RPL_001;
                break;
            case "002":
                command = IrcCommand2.RPL_002;
                break;
            case "003":
                command = IrcCommand2.RPL_003;
                break;
            case "004":
                command = IrcCommand2.RPL_004;
                break;
            case "353":
                command = IrcCommand2.RPL_353;
                break;
            case "366":
                command = IrcCommand2.RPL_366;
                break;
            case "372":
                command = IrcCommand2.RPL_372;
                break;
            case "375":
                command = IrcCommand2.RPL_375;
                break;
            case "376":
                command = IrcCommand2.RPL_376;
                break;
            case "WHISPER":
                command = IrcCommand2.Whisper;
                break;
            case "SERVERCHANGE":
                command = IrcCommand2.ServerChange;
                break;
            case "RECONNECT":
                command = IrcCommand2.Reconnect;
                break;
            case "ROOMSTATE":
                command = IrcCommand2.RoomState;
                break;
            case "USERNOTICE":
                command = IrcCommand2.UserNotice;
                break;
            case "MODE":
                command = IrcCommand2.Mode;
                break;
        }

        string parameters = raw.Substring(starts[(int)ParserState.STATE_PARAM],
            lens[(int)ParserState.STATE_PARAM]);
        string message = raw.Substring(starts[(int)ParserState.STATE_TRAILING],
            lens[(int)ParserState.STATE_TRAILING]);
        string hostmask = raw.Substring(starts[(int)ParserState.STATE_PREFIX],
            lens[(int)ParserState.STATE_PREFIX]);
        return new IrcMessage(command, new[] { parameters, message }, hostmask, tagDict);
    }

    /// <summary>
    /// Enum ParserState
    /// </summary>
    private enum ParserState
    {
        /// <summary>
        /// The state none
        /// </summary>
        STATE_NONE,
        /// <summary>
        /// The state v3
        /// </summary>
        STATE_V3,
        /// <summary>
        /// The state prefix
        /// </summary>
        STATE_PREFIX,
        /// <summary>
        /// The state command
        /// </summary>
        STATE_COMMAND,
        /// <summary>
        /// The state parameter
        /// </summary>
        STATE_PARAM,
        /// <summary>
        /// The state trailing
        /// </summary>
        STATE_TRAILING
    };

    public static void HandleIrcMessage(IrcMessage ircMessage)
    {
        if (ircMessage.Message.Contains("Login authentication failed"))
        {
            (TwitchLib, OnIncorrectLoginArgs) asdf = (default!, new OnIncorrectLoginArgs { Exception = new ErrorLoggingInException(ircMessage.ToString(), "occluder") });
            return;
        }
        switch (ircMessage.Command)
        {
            case IrcCommand2.PrivMsg:
                HandlePrivMsg(ircMessage);
                return;
            case IrcCommand2.Notice:
                //
                break;
            case IrcCommand2.Ping:
                //
                return;
            case IrcCommand2.Pong:
                return;
            case IrcCommand2.Join:
                //
                break;
            case IrcCommand2.Part:
                //
                break;
            case IrcCommand2.ClearChat:
                //
                break;
            case IrcCommand2.ClearMsg:
                break;
            case IrcCommand2.UserState:
                break;
            case IrcCommand2.GlobalUserState:
                break;
            case IrcCommand2.RPL_001:
                break;
            case IrcCommand2.RPL_002:
                break;
            case IrcCommand2.RPL_003:
                break;
            case IrcCommand2.RPL_004:
                //
                break;
            case IrcCommand2.RPL_353:
                break;
            case IrcCommand2.RPL_366:
                //
                break;
            case IrcCommand2.RPL_372:
                break;
            case IrcCommand2.RPL_375:
                break;
            case IrcCommand2.RPL_376:
                break;
            case IrcCommand2.Whisper:
                break;
            case IrcCommand2.RoomState:
                break;
            case IrcCommand2.Reconnect:
                break;
            case IrcCommand2.UserNotice:
                break;
            case IrcCommand2.Mode:
                break;
            case IrcCommand2.Cap:
                break;
            case IrcCommand2.Unknown:
            // fall through
            default:
                //
                break;
        }
    }

    private static void HandlePrivMsg(IrcMessage ircMessage)
    {
        ChatMessage chatMessage = new ChatMessage("occluder", ircMessage, ref _channelEmotes, false);

        var message = new OnMessageReceivedArgs { ChatMessage = chatMessage };

        if (ircMessage.Tags.TryGetValue(Tags.MsgId, out var msgId))
            if (msgId == MsgIds.UserIntro)
            {
                var intro = new OnUserIntroArgs { ChatMessage = chatMessage };
            }
    }
}

public class OnMessageReceivedArgs : EventArgs
{
    /// <summary>
    /// Property representing received chat message.
    /// </summary>
    public ChatMessage ChatMessage;
}

public class OnUserIntroArgs : EventArgs
{
    /// <summary>
    /// Property representing the PRIVMSG
    /// </summary>
    public ChatMessage ChatMessage;
}

public enum IrcCommand2
{
    Unknown,
    PrivMsg,
    Notice,
    Ping,
    Pong,
    Join,
    Part,
    ClearChat,
    ClearMsg,
    UserState,
    GlobalUserState,
    Nick,
    Pass,
    Cap,
    RPL_001,
    RPL_002,
    RPL_003,
    RPL_004,
    RPL_353,
    RPL_366,
    RPL_372,
    RPL_375,
    RPL_376,
    Whisper,
    RoomState,
    Reconnect,
    ServerChange,
    UserNotice,
    Mode
}

public class JoinedChannel
{
    /// <summary>The current channel the TwitcChatClient is connected to.</summary>
    public string Channel { get; }

    /// <summary>Object representing current state of channel (r9k, slow, etc).</summary>
    public ChannelState ChannelState { get; protected set; }

    /// <summary>The most recent message received.</summary>
    public ChatMessage PreviousMessage { get; protected set; }

    /// <summary>JoinedChannel object constructor.</summary>
    public JoinedChannel(string channel)
    {
        Channel = channel;
    }

    /// <summary>Handles a message</summary>
    public void HandleMessage(ChatMessage message)
    {
        PreviousMessage = message;
    }
}

public class ChannelState
{
    /// <summary>Property representing the current broadcaster language.</summary>
    public string BroadcasterLanguage { get; }

    /// <summary>Property representing the current channel.</summary>
    public string Channel { get; }

    /// <summary>Property representing whether EmoteOnly mode is being applied to chat or not. WILL BE NULL IF VALUE NOT PRESENT.</summary>
    public bool? EmoteOnly { get; }

    /// <summary>Property representing how long needed to be following to talk. If null, FollowersOnly is not enabled.</summary>
    public TimeSpan? FollowersOnly { get; } = null;

    /// <summary>Property representing mercury value. Not sure what it's for.</summary>
    public bool Mercury { get; }

    /// <summary>Property representing whether R9K is being applied to chat or not. WILL BE NULL IF VALUE NOT PRESENT.</summary>
    public bool? R9K { get; }

    /// <summary>Property representing whether Rituals is enabled or not. WILL BE NULL IF VALUE NOT PRESENT.</summary>
    public bool? Rituals { get; }

    /// <summary>Twitch assigned room id</summary>
    public string RoomId { get; }

    /// <summary>Property representing whether Slow mode is being applied to chat or not. WILL BE NULL IF VALUE NOT PRESENT.</summary>
    public int? SlowMode { get; }

    /// <summary>Property representing whether Sub Mode is being applied to chat or not. WILL BE NULL IF VALUE NOT PRESENT.</summary>
    public bool? SubOnly { get; }

    /// <summary>ChannelState object constructor.</summary>
    public ChannelState(IrcMessage ircMessage)
    {
        //@broadcaster-lang=;emote-only=0;r9k=0;slow=0;subs-only=1 :tmi.twitch.tv ROOMSTATE #burkeblack
        foreach (var tag in ircMessage.Tags.Keys)
        {
            var tagValue = ircMessage.Tags[tag];

            switch (tag)
            {
                case Tags.BroadcasterLang:
                    BroadcasterLanguage = tagValue;
                    break;
                case Tags.EmoteOnly:
                    EmoteOnly = tagValue == "1";
                    break;
                case Tags.R9K:
                    R9K = tagValue == "1";
                    break;
                case Tags.Rituals:
                    Rituals = tagValue == "1";
                    break;
                case Tags.Slow:
                    var success = int.TryParse(tagValue, out var slowDuration);
                    SlowMode = success ? slowDuration : (int?)null;
                    break;
                case Tags.SubsOnly:
                    SubOnly = tagValue == "1";
                    break;
                case Tags.FollowersOnly:
                    if (int.TryParse(tagValue, out int minutes) && minutes > -1)
                    {
                        FollowersOnly = TimeSpan.FromMinutes(minutes);
                    }
                    break;
                case Tags.RoomId:
                    RoomId = tagValue;
                    break;
                case Tags.Mercury:
                    Mercury = tagValue == "1";
                    break;
                default:
                    Console.WriteLine("[TwitchLib][ChannelState] Unaccounted for: " + tag);
                    break;
            }
        }
        Channel = ircMessage.Channel;
    }

    public ChannelState(
        bool r9k,
        bool rituals,
        bool subonly,
        int slowMode,
        bool emoteOnly,
        string broadcasterLanguage,
        string channel,
        TimeSpan followersOnly,
        bool mercury,
        string roomId)
    {
        R9K = r9k;
        Rituals = rituals;
        SubOnly = subonly;
        SlowMode = slowMode;
        EmoteOnly = emoteOnly;
        BroadcasterLanguage = broadcasterLanguage;
        Channel = channel;
        FollowersOnly = followersOnly;
        Mercury = mercury;
        RoomId = roomId;
    }
}

public class MessageEmoteCollection
{
    private readonly SortedList<string, MessageEmote> _emoteList;
    private const string BasePattern = @"(\b{0}\b)";

    /// <summary> Do not access directly! Backing field for <see cref="CurrentPattern"/> </summary>
    private string _currentPattern;
    private Regex _regex;
    private readonly EmoteFilterDelegate _preferredFilter;

    /// <summary>
    ///     Property so that we can be confident <see cref="PatternChanged"/>
    ///     always reflects changes to <see cref="CurrentPattern"/>.
    /// </summary>
    private string CurrentPattern
    {
        get => _currentPattern;
        set
        {
            if (_currentPattern != null && _currentPattern.Equals(value))
                return;
            _currentPattern = value;
            PatternChanged = true;
        }
    }

    private Regex CurrentRegex
    {
        get
        {
            if (PatternChanged)
            {
                if (CurrentPattern != null)
                {
                    _regex = new Regex(string.Format(CurrentPattern, ""));
                    PatternChanged = false;
                }
                else
                {
                    _regex = null;
                }
            }
            return _regex;
        }
    }

    private bool PatternChanged { get; set; }

    private EmoteFilterDelegate CurrentEmoteFilter { get; set; } = AllInclusiveEmoteFilter;

    /// <summary>
    ///     Default, empty constructor initializes the list and sets the preferred
    ///     <see cref="EmoteFilterDelegate"/> to <see cref="AllInclusiveEmoteFilter(MessageEmote)"/>
    /// </summary>
    public MessageEmoteCollection()
    {
        _emoteList = new SortedList<string, MessageEmote>();
        _preferredFilter = AllInclusiveEmoteFilter;
    }

    /// <inheritdoc />
    /// <summary>
    ///     Constructor which specifies a particular preferred <see cref="T:TwitchLib.Models.Client.MessageEmoteCollection.EmoteFilterDelegate" />
    /// </summary>
    /// <param name="preferredFilter"></param>
    public MessageEmoteCollection(EmoteFilterDelegate preferredFilter) : this()
    {
        _preferredFilter = preferredFilter;
    }

    /// <summary>
    ///     Adds an <see cref="MessageEmote"/> to the collection. Duplicate emotes
    ///     (judged by <see cref="MessageEmote.Text"/>) are ignored.
    /// </summary>
    /// <param name="emote">The <see cref="MessageEmote"/> to add to the collection.</param>
    public void Add(MessageEmote emote)
    {
        if (!_emoteList.TryGetValue(emote.Text, out var _))
        {
            _emoteList.Add(emote.Text, emote);
        }

        if (CurrentPattern == null)
        {
            //string i = String.Format(_basePattern, "(" + emote.EscapedText + "){0}");
            CurrentPattern = string.Format(BasePattern, emote.EscapedText);
        }
        else
        {
            CurrentPattern = CurrentPattern + "|" + string.Format(BasePattern, emote.EscapedText);
        }
    }

    /// <summary>
    ///     Adds every <see cref="MessageEmote"/> from an <see cref="IEnumerable{T}">enumerable</see>
    ///     collection to the internal collection.
    ///     Duplicate emotes (judged by <see cref="MessageEmote.Text"/>) are ignored.
    /// </summary>
    /// <param name="emotes">A collection of <see cref="MessageEmote"/>s.</param>
    public void Merge(IEnumerable<MessageEmote> emotes)
    {
        var enumerator = emotes.GetEnumerator();
        while (enumerator.MoveNext())
        {
            Add(enumerator.Current);
        }

        enumerator.Dispose();
    }

    /// <summary>
    ///     Removes the specified <see cref="MessageEmote"/> from the collection.
    /// </summary>
    /// <param name="emote">The <see cref="MessageEmote"/> to remove.</param>
    public void Remove(MessageEmote emote)
    {
        if (!_emoteList.ContainsKey(emote.Text))
            return;

        _emoteList.Remove(emote.Text);

        // These patterns look a lot scarier than they are because we have to look for
        // a lot of regex characters, which means we do a lot of escaping!

        // Matches ^(\bEMOTE\b)| and ^(\bEMOTE\b)
        // It's all grouped so that we can OR it with the second pattern.
        var firstEmotePattern = @"(^\(\\b" + emote.EscapedText + @"\\b\)\|?)";
        // Matches |(\bEMOTE\b) including the preceding | so that the following | and emote (if any)
        // merge seamlessly when this section is removed. Again, wrapped in a group.
        var otherEmotePattern = @"(\|\(\\b" + emote.EscapedText + @"\\b\))";
        var newPattern = Regex.Replace(CurrentPattern, firstEmotePattern + "|" + otherEmotePattern, "");
        CurrentPattern = newPattern.Equals("") ? null : newPattern;
    }

    /// <summary>
    ///     Removes all <see cref="MessageEmote"/>s from the collection.
    /// </summary>
    public void RemoveAll()
    {
        _emoteList.Clear();
        CurrentPattern = null;
    }

    /// <summary>
    ///     Replaces all instances of all registered emotes passing the provided
    ///     <see cref="EmoteFilterDelegate"/> with their designated
    ///     <see cref="MessageEmote.ReplacementString"/>s
    /// </summary>
    /// <param name="originalMessage">
    ///     The original message which needs to be processed for emotes.
    /// </param>
    /// <param name="del">
    ///     An <see cref="EmoteFilterDelegate"/> which returns true if its
    ///     received <see cref="MessageEmote"/> is to be replaced.
    ///     Defaults to <see cref="CurrentEmoteFilter"/>.
    /// </param>
    /// <returns>
    ///     A string where all of the original emote text has been replaced with
    ///     its designated <see cref="MessageEmote.ReplacementString"/>s
    /// </returns>
    public string ReplaceEmotes(string originalMessage, EmoteFilterDelegate del = null)
    {
        if (CurrentRegex == null)
            return originalMessage;
        if (del != null && del != CurrentEmoteFilter)
            CurrentEmoteFilter = del;
        var newMessage = CurrentRegex.Replace(originalMessage, GetReplacementString);
        CurrentEmoteFilter = _preferredFilter;
        return newMessage;
    }

    /// <summary>
    ///     A delegate function which, when given a <see cref="MessageEmote"/>,
    ///     determines whether it should be replaced.
    /// </summary>
    /// <param name="emote">The <see cref="MessageEmote"/> to be considered</param>
    /// <returns>true if the <see cref="MessageEmote"/> should be replaced.</returns>
    public delegate bool EmoteFilterDelegate(MessageEmote emote);

    /// <summary>
    ///     The default emote filter includes every <see cref="MessageEmote"/> registered on this list.
    /// </summary>
    /// <param name="emote">An emote which is ignored in this filter.</param>
    /// <returns>true always</returns>
    public static bool AllInclusiveEmoteFilter(MessageEmote emote)
    {
        return true;
    }

    /// <summary>
    ///     This emote filter includes only <see cref="MessageEmote"/>s provided by Twitch.
    /// </summary>
    /// <param name="emote">
    ///     A <see cref="MessageEmote"/> which will be replaced if its
    ///     <see cref="MessageEmote.Source">Source</see> is <see cref="MessageEmote.EmoteSource.Twitch"/>
    /// </param>
    /// <returns>true always</returns>
    public static bool TwitchOnlyEmoteFilter(MessageEmote emote)
    {
        return emote.Source == MessageEmote.EmoteSource.Twitch;
    }

    private string GetReplacementString(Match m)
    {
        if (!_emoteList.ContainsKey(m.Value))
            return m.Value;

        var emote = _emoteList[m.Value];
        return CurrentEmoteFilter(emote) ? emote.ReplacementString : m.Value;
        //If the match doesn't exist in the list ("shouldn't happen") or the filter excludes it, don't replace.
    }
}

public class MessageEmote
{
    /// <summary>
    ///     Delegate allowing Emotes to handle their replacement text on a case-by-case basis.
    /// </summary>
    /// <returns>The string for the calling emote to be replaced with.</returns>
    public delegate string ReplaceEmoteDelegate(MessageEmote caller);

    /// <summary>
    ///     Collection of Composite Format Strings which will substitute an
    ///     emote ID to get a URL for an image from the Twitch CDN
    /// </summary>
    /// <remarks>
    ///     These are sorted such that the <see cref="EmoteSize"/> enum can be used as an index,
    ///     eg TwitchEmoteUrls[<see cref="EmoteSize.Small"/>]
    /// </remarks>
    public static readonly ReadOnlyCollection<string> TwitchEmoteUrls = new ReadOnlyCollection<string>(
        new[]
        {
            "https://static-cdn.jtvnw.net/emoticons/v1/{0}/1.0",
            "https://static-cdn.jtvnw.net/emoticons/v1/{0}/2.0",
            "https://static-cdn.jtvnw.net/emoticons/v1/{0}/3.0"
        }
    );

    #region Third-Party Emote URLs
    //As this is a Twitch Library these could understandably be removed, but they are rather handy

    /// <summary>
    ///     Collection of Composite Format Strings which will substitute an
    ///     emote ID to get a URL for an image from the FFZ CDN
    /// </summary>
    /// <remarks>
    ///     These are sorted such that the <see cref="EmoteSize"/> enum can be used as an index,
    ///     eg FrankerFaceZEmoteUrls[<see cref="EmoteSize.Small"/>]
    ///     WARNING: FrankerFaceZ does not require users to submit all sizes,
    ///     so using something other than Small images may result in broken links!
    /// </remarks>
    public static readonly ReadOnlyCollection<string> FrankerFaceZEmoteUrls = new ReadOnlyCollection<string>(
        new[]
        {
            "//cdn.frankerfacez.com/emoticon/{0}/1",
            "//cdn.frankerfacez.com/emoticon/{0}/2",
            "//cdn.frankerfacez.com/emoticon/{0}/4"
        }
    );

    /// <summary>
    ///     Collection of Composite Format Strings which will substitute
    ///     an emote ID to get a URL for an image from the BTTV CDN
    ///     </summary>
    /// <remarks>
    ///     These are sorted such that the <see cref="EmoteSize"/> enum can be used as an index,
    ///     eg BetterTwitchTvEmoteUrls[<see cref="EmoteSize.Small"/>]
    /// </remarks>
    public static readonly ReadOnlyCollection<string> BetterTwitchTvEmoteUrls = new ReadOnlyCollection<string>(
        new[]
        {
            "//cdn.betterttv.net/emote/{0}/1x",
            "//cdn.betterttv.net/emote/{0}/2x",
            "//cdn.betterttv.net/emote/{0}/3x"
        }
    );
    #endregion Third-Party Emote URLs

    /// <summary>
    ///     A delegate which attempts to match the calling <see cref="MessageEmote"/> with its
    ///     <see cref="EmoteSource"/> and pulls the <see cref="EmoteSize.Small">small</see> version
    ///     of the URL.
    /// </summary>
    /// <param name="caller"></param>
    /// <returns></returns>
    public static string SourceMatchingReplacementText(MessageEmote caller)
    {
        var sizeIndex = (int)caller.Size;
        switch (caller.Source)
        {
            case EmoteSource.BetterTwitchTv:
                return string.Format(BetterTwitchTvEmoteUrls[sizeIndex], caller.Id);
            case EmoteSource.FrankerFaceZ:
                return string.Format(FrankerFaceZEmoteUrls[sizeIndex], caller.Id);
            case EmoteSource.Twitch:
                return string.Format(TwitchEmoteUrls[sizeIndex], caller.Id);
        }
        return caller.Text;
    }

    /// <summary> Enum supplying the supported sites which provide Emote images.</summary>
    public enum EmoteSource
    {
        /// <summary>Emotes hosted by Twitch.tv natively</summary>
        Twitch,

        /// <summary>Emotes hosted by FrankerFaceZ.com</summary>
        FrankerFaceZ,

        /// <summary>Emotes hosted by BetterTTV.net</summary>
        BetterTwitchTv
    }

    /// <summary> Enum denoting the emote sizes</summary>
    public enum EmoteSize
    {
        /// <summary>
        ///     Best support
        ///     Small-sized emotes are the standard size used in the Twitch web chat.
        /// </summary>
        Small = 0,

        /// <summary>
        ///     Medium-sized emotes are not supported by all browsers, and
        ///     FrankerFaceZ does not require emotes to be submitted in this size
        /// </summary>
        Medium = 1,

        /// <summary>
        ///     Large-sized emotes are not supported by all browsers, and
        ///     FrankerFaceZ does not require emotes to be submitted in this size
        ///     </summary>
        Large = 2
    }

    private readonly string _id, _text, _escapedText;
    private readonly EmoteSource _source;
    private readonly EmoteSize _size;

    /// <summary>
    ///     Emote ID as used by the emote source. Will be provided as {0}
    ///     to be substituted into the indicated URL if needed.
    /// </summary>
    public string Id => _id;

    /// <summary>
    ///     Emote text which appears in a message and is meant to be replaced by the emote image.
    /// </summary>
    public string Text => _text;

    /// <summary>
    ///     The specified <see cref="EmoteSource"/> for this emote.
    /// </summary>
    public EmoteSource Source => _source;

    /// <summary>
    ///     The specified <see cref="EmoteSize"/> for this emote.
    /// </summary>
    public EmoteSize Size => _size;

    /// <summary>
    ///    The string to substitute emote text for.
    /// </summary>
    public string ReplacementString => ReplacementDelegate(this);

    /// <summary>
    ///     The desired <see cref="ReplaceEmoteDelegate"/> to use for replacing text in a given emote.
    ///     Default: <see cref="SourceMatchingReplacementText(MessageEmote)"/>
    /// </summary>
    public static ReplaceEmoteDelegate ReplacementDelegate { get; set; } = SourceMatchingReplacementText;

    /// <summary>
    ///     The emote text <see cref="Regex.Escape(string)">regex-escaped</see>
    ///     so that it can be embedded into a regex pattern.
    /// </summary>
    public string EscapedText => _escapedText;

    /// <summary>
    ///     Constructor for a new MessageEmote instance.
    /// </summary>
    /// <param name="id">
    ///     The unique identifier which the emote provider uses to generate CDN URLs.
    /// </param>
    /// <param name="text">
    ///     The string which users type to create this emote in chat.
    /// </param>
    /// <param name="source">
    ///     An <see cref="EmoteSource"/> where an image can be found for this emote.
    ///     Default: <see cref="EmoteSource.Twitch"/>
    /// </param>
    /// <param name="size">
    ///     An <see cref="EmoteSize"/> to pull for this image.
    ///     Default: <see cref="EmoteSize.Small"/>
    /// </param>
    /// <param name="replacementDelegate">
    ///     A string (optionally Composite Format with "{0}" representing
    ///     <paramref name="id"/>) which will be used instead of any of the emote URLs.
    ///     Default: null
    /// </param>
    public MessageEmote(
        string id,
        string text,
        EmoteSource source = EmoteSource.Twitch,
        EmoteSize size = EmoteSize.Small,
        ReplaceEmoteDelegate replacementDelegate = null)
    {
        _id = id;
        _text = text;
        _escapedText = Regex.Escape(text);
        _source = source;
        _size = size;
        if (replacementDelegate != null)
        {
            ReplacementDelegate = replacementDelegate;
        }
    }
}

public interface IBuilder<T>
{
    T Build();
}

public sealed class EmoteBuilder : IBuilder<Emote>
{
    private string _id;
    private string _name;
    private int _startIndex;
    private int _endIndex;

    private EmoteBuilder()
    {
    }

    public static EmoteBuilder Create()
    {
        return new EmoteBuilder();
    }

    public EmoteBuilder WithId(string id)
    {
        _id = id;
        return this;
    }

    public EmoteBuilder WithName(string name)
    {
        _name = name;
        return this;
    }

    public EmoteBuilder WithStartIndex(int startIndex)
    {
        _startIndex = startIndex;
        return this;
    }

    public EmoteBuilder WithEndIndex(int endIndex)
    {
        _endIndex = endIndex;
        return this;
    }

    public Emote Build()
    {
        return new Emote(
            _id,
            _name,
            _startIndex,
            _endIndex);
    }
}

public class EmoteSet
{
    /// <summary>List containing all emotes in the message.</summary>
    public List<Emote> Emotes { get; }

    /// <summary>The raw emote set string obtained from Twitch, for legacy purposes.</summary>
    public string RawEmoteSetString { get; }

    /// <summary>Constructor for ChatEmoteSet object.</summary>
    /// <param name="rawEmoteSetString"></param>
    /// <param name="message"></param>
    public EmoteSet(string rawEmoteSetString, string message)
    {
        // this should be removed and used outside of object
        RawEmoteSetString = rawEmoteSetString;
        EmoteExtractor emoteExtractor = new EmoteExtractor();
        Emotes = emoteExtractor.Extract(rawEmoteSetString, message).ToList();
    }

    public class EmoteExtractor
    {
        public IEnumerable<Emote> Extract(string rawEmoteSetString, string message)
        {
            if (string.IsNullOrEmpty(rawEmoteSetString)
               || string.IsNullOrEmpty(message))
            {
                yield break;
            }

            if (rawEmoteSetString.Contains("/"))
            {
                // Message contains multiple different emotes, first parse by unique emotes: 28087:15-21/25:5-9,28-32
                foreach (var emoteData in rawEmoteSetString.Split('/'))
                {
                    var emoteId = emoteData.Split(':')[0];
                    if (emoteData.Contains(","))
                    {
                        // Multiple copies of a single emote: 25:5-9,28-32
                        foreach (var emote in emoteData.Replace($"{emoteId}:", "").Split(','))
                            yield return GetEmote(emote, emoteId, message);
                    }
                    else
                    {
                        // Single copy of single emote: 25:5-9/28087:16-22
                        yield return GetEmote(emoteData, emoteId, message, true);
                    }
                }
            }
            else
            {
                var emoteId = rawEmoteSetString.Split(':')[0];
                // Message contains a single, or multiple of the same emote
                if (rawEmoteSetString.Contains(","))
                {
                    // Multiple copies of a single emote: 25:5-9,28-32
                    foreach (var emote in rawEmoteSetString.Replace($"{emoteId}:", "").Split(','))
                        yield return GetEmote(emote, emoteId, message);
                }
                else
                {
                    // Single copy of single emote: 25:5-9
                    yield return GetEmote(rawEmoteSetString, emoteId, message, true);
                }
            }
        }

        private Emote GetEmote(string emoteData, string emoteId, string message, bool single = false)
        {
            int startIndex = -1;
            int endIndex = -1;

            if (single)
            {
                startIndex = int.Parse(emoteData.Split(':')[1].Split('-')[0]);
                endIndex = int.Parse(emoteData.Split(':')[1].Split('-')[1]);
            }
            else
            {
                startIndex = int.Parse(emoteData.Split('-')[0]);
                endIndex = int.Parse(emoteData.Split('-')[1]);
            }

            string name = message.Substring(startIndex, (endIndex - startIndex) + 1);

            EmoteBuilder emoteBuilder = EmoteBuilder.Create()
                                                    .WithId(emoteId)
                                                    .WithName(name)
                                                    .WithStartIndex(startIndex)
                                                    .WithEndIndex(endIndex);

            return emoteBuilder.Build();
        }
    }

    /// <summary>Constructor for ChatEmoteSet object.</summary>
    /// <param name="emotes">Collection of Emote instances</param>
    /// <param name="rawEmoteSetString">Original string from which emotes were created</param>
    public EmoteSet(IEnumerable<Emote> emotes, string emoteSetData)
    {
        RawEmoteSetString = emoteSetData;
        Emotes = emotes.ToList();
    }
}

public class Emote
{
    /// <summary>Twitch-assigned emote Id.</summary>
    public string Id { get; }

    /// <summary>The name of the emote. For example, if the message was "This is Kappa test.", the name would be 'Kappa'.</summary>
    public string Name { get; }

    /// <summary>Character starting index. For example, if the message was "This is Kappa test.", the start index would be 8 for 'Kappa'.</summary>
    public int StartIndex { get; }

    /// <summary>Character ending index. For example, if the message was "This is Kappa test.", the start index would be 12 for 'Kappa'.</summary>
    public int EndIndex { get; }

    /// <summary>URL to Twitch hosted emote image.</summary>
    public string ImageUrl { get; }

    /// <summary>
    /// Emote constructor.
    /// </summary>
    /// <param name="emoteId"></param>
    /// <param name="name"></param>
    /// <param name="emoteStartIndex"></param>
    /// <param name="emoteEndIndex"></param>
    public Emote(
        string emoteId,
        string name,
        int emoteStartIndex,
        int emoteEndIndex)
    {
        Id = emoteId;
        Name = name;
        StartIndex = emoteStartIndex;
        EndIndex = emoteEndIndex;
        ImageUrl = $"https://static-cdn.jtvnw.net/emoticons/v1/{emoteId}/1.0";
    }
}

public abstract class TwitchLibMessage
{
    /// <summary>List of key-value pair badges.</summary>
    public List<KeyValuePair<string, string>> Badges { get; protected set; }

    /// <summary>Twitch username of the bot that received the message.</summary>
    public string BotUsername { get; protected set; }

    /// <summary>Property representing HEX color as a System.Drawing.Color object.</summary>
    public Color Color { get; protected set; }

    /// <summary>Hex representation of username color in chat (THIS CAN BE NULL IF VIEWER HASN'T SET COLOR).</summary>
    public string ColorHex { get; protected set; }

    /// <summary>Case-sensitive username of sender of chat message.</summary>
    public string DisplayName { get; protected set; }

    /// <summary>Emote Ids that exist in message.</summary>
    public EmoteSet EmoteSet { get; protected set; }

    /// <summary>Twitch site-wide turbo status.</summary>
    public bool IsTurbo { get; protected set; }

    /// <summary>Twitch-unique integer assigned on per account basis.</summary>
    public string UserId { get; protected set; }

    /// <summary>Username of sender of chat message.</summary>
    public string Username { get; protected set; }

    /// <summary>User type can be viewer, moderator, global mod, admin, or staff</summary>
    public UserType UserType { get; protected set; }

    /// <summary>Raw IRC-style text received from Twitch.</summary>
    public string RawIrcMessage { get; protected set; }
}

public enum UserType : byte
{
    /// <summary>The standard user-type representing a standard viewer.</summary>
    Viewer,
    /// <summary>User-type representing viewers with channel-specific moderation powers.</summary>
    Moderator,
    /// <summary>User-type representing viewers with Twitch-wide moderation powers.</summary>
    GlobalModerator,
    /// <summary>User-type representing the broadcaster of the channel</summary>
    Broadcaster,
    /// <summary>User-type representing viewers with Twitch-wide moderation powers that are paid.</summary>
    Admin,
    /// <summary>User-type representing viewers that are Twitch employees.</summary>
    Staff
}

public class CheerBadge
{
    /// <summary>Property representing raw cheer amount represented by badge.</summary>
    public int CheerAmount { get; }

    /// <summary>Property representing the color of badge via an enum.</summary>
    public BadgeColor Color { get; }

    /// <summary>Constructor for CheerBadge</summary>
    public CheerBadge(int cheerAmount)
    {
        CheerAmount = cheerAmount;
        Color = GetColor(cheerAmount);
    }

    private BadgeColor GetColor(int cheerAmount)
    {
        if (cheerAmount >= 10000)
            return BadgeColor.Red;
        if (cheerAmount >= 5000)
            return BadgeColor.Blue;
        if (cheerAmount >= 1000)
            return BadgeColor.Green;
        return cheerAmount >= 100 ? BadgeColor.Purple : BadgeColor.Gray;
    }
}

public enum BadgeColor
{
    /// <summary>Red = 10000+</summary>
    Red = 10000,
    /// <summary>Blue = 5000 -> 9999</summary>
    Blue = 5000,
    /// <summary>Green = 1000 -> 4999</summary>
    Green = 1000,
    /// <summary>Purple = 100 -> 999</summary>
    Purple = 100,
    /// <summary>Gray = 1 -> 99</summary>
    Gray = 1
}

public class ChatMessage : TwitchLibMessage
{
    protected readonly MessageEmoteCollection _emoteCollection;

    /// <summary>Information associated with badges. Not all badges will be in this list. Use carefully.</summary>
    public List<KeyValuePair<string, string>> BadgeInfo { get; }

    /// <summary>If viewer sent bits in their message, total amount will be here.</summary>
    public int Bits { get; }

    /// <summary>Number of USD (United States Dollars) spent on bits.</summary>
    public double BitsInDollars { get; }

    /// <summary>Twitch channel message was sent from (useful for multi-channel bots).</summary>
    public string Channel { get; }

    /// <summary>If a cheer badge exists, this property represents the raw value and color (more later). Can be null.</summary>
    public CheerBadge CheerBadge { get; }

    /// <summary>If a custom reward is present with the message, the ID will be set (null by default)</summary>
    public string CustomRewardId { get; }

    /// <summary>Text after emotes have been handled (if desired). Will be null if replaceEmotes is false.</summary>
    public string EmoteReplacedMessage { get; }

    /// <summary>Unique message identifier assigned by Twitch</summary>
    public string Id { get; }

    /// <summary>Chat message from broadcaster identifier flag</summary>
    public bool IsBroadcaster { get; }

    /// <summary>Chat message is the first message, ever, from this user in this chat</summary>
    public bool IsFirstMessage { get; }

    /// <summary>Chat message is highlighted in chat via channel points</summary>
    public bool IsHighlighted { get; internal set; }

    /// <summary>Chat message /me identifier flag.</summary>
    public bool IsMe { get; }

    /// <summary>Channel specific moderator status.</summary>
    public bool IsModerator { get; }

    /// <summary>Message used channel points to skip sub mode</summary>
    public bool IsSkippingSubMode { get; internal set; }

    /// <summary>Channel specific subscriber status.</summary>
    public bool IsSubscriber { get; }

    /// <summary>Message is from channel VIP</summary>
    public bool IsVip { get; }

    /// <summary>Message is from a Twitch Staff member</summary>
    public bool IsStaff { get; }

    /// <summary>Message is from a Twitch Partner</summary>
    public bool IsPartner { get; }

    /// <summary>Twitch chat message contents.</summary>
    public string Message { get; }

    /// <summary>Experimental property noisy determination by Twitch.</summary>
    public Noisy Noisy { get; }

    /// <summary>Unique identifier of chat room.</summary>
    public string RoomId { get; }

    /// <summary>Number of months a person has been subbed.</summary>
    public int SubscribedMonthCount { get; }

    /// <summary>Sent timestamp generated by TMI</summary>
    public string TmiSentTs { get; }

    // <summary>Chat reply information. Will be null if it is not a reply.</summary>
    public ChatReply ChatReply { get; }

    public static List<KeyValuePair<string, string>> ParseBadges(string badgesStr)
    {
        var badges = new List<KeyValuePair<string, string>>();

        if (badgesStr.Contains('/'))
        {
            if (!badgesStr.Contains(","))
                badges.Add(new KeyValuePair<string, string>(badgesStr.Split('/')[0], badgesStr.Split('/')[1]));
            else
                foreach (var badge in badgesStr.Split(','))
                    badges.Add(new KeyValuePair<string, string>(badge.Split('/')[0], badge.Split('/')[1]));
        }

        return badges;
    }

    //Example IRC message: @badges=moderator/1,warcraft/alliance;color=;display-name=Swiftyspiffyv4;emotes=;mod=1;room-id=40876073;subscriber=0;turbo=0;user-id=103325214;user-type=mod :swiftyspiffyv4!swiftyspiffyv4@swiftyspiffyv4.tmi.twitch.tv PRIVMSG #swiftyspiffy :asd
    /// <summary>Constructor for ChatMessage object.</summary>
    /// <param name="botUsername">The username of the bot that received the message.</param>
    /// <param name="ircMessage">The IRC message from Twitch to be processed.</param>
    /// <param name="emoteCollection">The <see cref="MessageEmoteCollection"/> to register new emotes on and, if desired, use for emote replacement.</param>
    /// <param name="replaceEmotes">Whether to replace emotes for this chat message. Defaults to false.</param>
    public ChatMessage(
        string botUsername,
        IrcMessage ircMessage,
        ref MessageEmoteCollection emoteCollection,
        bool replaceEmotes = false)
    {
        BotUsername = botUsername;
        RawIrcMessage = ircMessage.ToString();
        Message = ircMessage.Message;

        if (Message.Length > 0 && (byte)Message[0] == 1 && (byte)Message[Message.Length - 1] == 1)
        {
            //Actions (/me {action}) are wrapped by byte=1 and prepended with "ACTION "
            //This setup clears all of that leaving just the action's text.
            //If you want to clear just the nonstandard bytes, use:
            //_message = _message.Substring(1, text.Length-2);
            if (Message.StartsWith("\u0001ACTION ") && Message.EndsWith("\u0001"))
            {
                Message = Message.Trim('\u0001').Substring(7);
                IsMe = true;
            }
        }

        _emoteCollection = emoteCollection;

        Username = ircMessage.User;
        Channel = ircMessage.Channel;

        foreach (var tag in ircMessage.Tags.Keys)
        {
            var tagValue = ircMessage.Tags[tag];

            switch (tag)
            {
                case Tags.Badges:
                    Badges = ParseBadges(tagValue);
                    // Iterate through saved badges for special circumstances
                    foreach (var badge in Badges)
                    {
                        switch (badge.Key)
                        {
                            case "bits":
                                CheerBadge = new CheerBadge(int.Parse(badge.Value));
                                break;
                            case "subscriber":
                                // Prioritize BadgeInfo subscribe count, as its more accurate
                                if (SubscribedMonthCount == 0)
                                {
                                    SubscribedMonthCount = int.Parse(badge.Value);
                                }
                                break;
                            case "vip":
                                IsVip = true;
                                break;
                            case "admin":
                                IsStaff = true;
                                break;
                            case "staff":
                                IsStaff = true;
                                break;
                            case "partner":
                                IsPartner = true;
                                break;

                        }
                    }
                    break;
                case Tags.BadgeInfo:
                    BadgeInfo = ParseBadges(tagValue);
                    // check if founder is one of them, and get months from that
                    var founderBadge = BadgeInfo.Find(b => b.Key == "founder");
                    if (!founderBadge.Equals(default(KeyValuePair<string, string>)))
                    {
                        IsSubscriber = true;
                        SubscribedMonthCount = int.Parse(founderBadge.Value);
                    }
                    else
                    {
                        var subBadge = BadgeInfo.Find(b => b.Key == "subscriber");
                        // BadgeInfo has better accuracy than Badges subscriber value
                        if (!subBadge.Equals(default(KeyValuePair<string, string>)))
                        {
                            SubscribedMonthCount = int.Parse(subBadge.Value);
                        }
                    }
                    break;
                case Tags.Bits:
                    Bits = int.Parse(tagValue);
                    BitsInDollars = ConvertBitsToUsd(Bits);
                    break;
                case Tags.Color:
                    ColorHex = tagValue;
                    if (!string.IsNullOrWhiteSpace(ColorHex))
                        Color = ColorTranslator.FromHtml(ColorHex);
                    break;
                case Tags.CustomRewardId:
                    CustomRewardId = tagValue;
                    break;
                case Tags.DisplayName:
                    DisplayName = tagValue;
                    break;
                case Tags.Emotes:
                    EmoteSet = new EmoteSet(tagValue, Message);
                    break;
                case Tags.FirstMessage:
                    IsFirstMessage = tagValue == "1";
                    break;
                case Tags.Id:
                    Id = tagValue;
                    break;
                case Tags.MsgId:
                    handleMsgId(tagValue);
                    break;
                case Tags.Mod:
                    IsModerator = tagValue == "1";
                    break;
                case Tags.Noisy:
                    Noisy = tagValue == "1" ? Noisy.True : Noisy.False;
                    break;
                case Tags.ReplyParentDisplayName:
                    if (ChatReply == null)
                    { ChatReply = new ChatReply(); } // ChatReply is null if not reply
                    ChatReply.ParentDisplayName = tagValue;
                    break;
                case Tags.ReplyParentMsgBody:
                    if (ChatReply == null)
                    { ChatReply = new ChatReply(); } // ChatReply is null if not reply
                    ChatReply.ParentMsgBody = tagValue;
                    break;
                case Tags.ReplyParentMsgId:
                    if (ChatReply == null)
                    { ChatReply = new ChatReply(); } // ChatReply is null if not reply
                    ChatReply.ParentMsgId = tagValue;
                    break;
                case Tags.ReplyParentUserId:
                    if (ChatReply == null)
                    { ChatReply = new ChatReply(); } // ChatReply is null if not reply
                    ChatReply.ParentUserId = tagValue;
                    break;
                case Tags.ReplyParentUserLogin:
                    if (ChatReply == null)
                    { ChatReply = new ChatReply(); } // ChatReply is null if not reply
                    ChatReply.ParentUserLogin = tagValue;
                    break;
                case Tags.RoomId:
                    RoomId = tagValue;
                    break;
                case Tags.Subscriber:
                    // this check because when founder is set, the subscriber value is actually 0, which is problematic
                    IsSubscriber = IsSubscriber == false ? tagValue == "1" : true;
                    break;
                case Tags.TmiSentTs:
                    TmiSentTs = tagValue;
                    break;
                case Tags.Turbo:
                    IsTurbo = tagValue == "1";
                    break;
                case Tags.UserId:
                    UserId = tagValue;
                    break;
                case Tags.UserType:
                    switch (tagValue)
                    {
                        case "mod":
                            UserType = UserType.Moderator;
                            break;
                        case "global_mod":
                            UserType = UserType.GlobalModerator;
                            break;
                        case "admin":
                            UserType = UserType.Admin;
                            IsStaff = true;
                            break;
                        case "staff":
                            UserType = UserType.Staff;
                            IsStaff = true;
                            break;
                        default:
                            UserType = UserType.Viewer;
                            break;
                    }
                    break;
            }
        }

        //Parse the emoteSet
        if (EmoteSet != null && Message != null && EmoteSet.Emotes.Count > 0)
        {
            var uniqueEmotes = EmoteSet.RawEmoteSetString.Split('/');
            foreach (var emote in uniqueEmotes)
            {
                var firstColon = emote.IndexOf(':');
                var firstComma = emote.IndexOf(',');
                if (firstComma == -1)
                    firstComma = emote.Length;
                var firstDash = emote.IndexOf('-');
                if (firstColon > 0 && firstDash > firstColon && firstComma > firstDash)
                {
                    if (int.TryParse(emote.Substring(firstColon + 1, firstDash - firstColon - 1), out var low) &&
                        int.TryParse(emote.Substring(firstDash + 1, firstComma - firstDash - 1), out var high))
                    {
                        if (low >= 0 && low < high && high < Message.Length)
                        {
                            //Valid emote, let's parse
                            var id = emote.Substring(0, firstColon);
                            //Pull the emote text from the message
                            var text = Message.Substring(low, high - low + 1);
                            _emoteCollection.Add(new MessageEmote(id, text));
                        }
                    }
                }
            }
            if (replaceEmotes)
            {
                EmoteReplacedMessage = _emoteCollection.ReplaceEmotes(Message);
            }
        }

        if (EmoteSet == null)
            EmoteSet = new EmoteSet(default(string), Message);

        // Check if display name was set, and if it wasn't, set it to username
        if (string.IsNullOrEmpty(DisplayName))
            DisplayName = Username;

        // Check if message is from broadcaster
        if (string.Equals(Channel, Username, StringComparison.InvariantCultureIgnoreCase))
        {
            UserType = UserType.Broadcaster;
            IsBroadcaster = true;
        }

        if (Channel.Split(':').Length == 3)
        {
            if (string.Equals(Channel.Split(':')[1], UserId, StringComparison.InvariantCultureIgnoreCase))
            {
                UserType = UserType.Broadcaster;
                IsBroadcaster = true;
            }
        }
    }


    public ChatMessage(
        string botUsername,
        string userId,
        string userName,
        string displayName,
        string colorHex,
        Color color,
        EmoteSet emoteSet,
        string message,
        UserType userType,
        string channel,
        string id,
        bool isSubscriber,
        int subscribedMonthCount,
        string roomId,
        bool isTurbo,
        bool isModerator,
        bool isMe,
        bool isBroadcaster,
        bool isVip,
        bool isPartner,
        bool isStaff,
        Noisy noisy,
        string rawIrcMessage,
        string emoteReplacedMessage,
        List<KeyValuePair<string, string>> badges,
        CheerBadge cheerBadge,
        int bits,
        double bitsInDollars)
    {
        BotUsername = botUsername;
        UserId = userId;
        DisplayName = displayName;
        ColorHex = colorHex;
        Color = color;
        EmoteSet = emoteSet;
        Message = message;
        UserType = userType;
        Channel = channel;
        Id = id;
        IsSubscriber = isSubscriber;
        SubscribedMonthCount = subscribedMonthCount;
        RoomId = roomId;
        IsTurbo = isTurbo;
        IsModerator = isModerator;
        IsMe = isMe;
        IsBroadcaster = isBroadcaster;
        IsVip = isVip;
        IsPartner = isPartner;
        IsStaff = isStaff;
        Noisy = noisy;
        RawIrcMessage = rawIrcMessage;
        EmoteReplacedMessage = emoteReplacedMessage;
        Badges = badges;
        CheerBadge = cheerBadge;
        Bits = bits;
        BitsInDollars = bitsInDollars;
        Username = userName;
    }

    private void handleMsgId(string val)
    {
        switch (val)
        {
            case MsgIds.HighlightedMessage:
                IsHighlighted = true;
                break;
            case MsgIds.SkipSubsModeMessage:
                IsSkippingSubMode = true;
                break;
        }
    }

    private static double ConvertBitsToUsd(int bits)
    {
        /*
        Conversion Rates
        100 bits = $1.40
        500 bits = $7.00
        1500 bits = $19.95 (5%)
        5000 bits = $64.40 (8%)
        10000 bits = $126.00 (10%)
        25000 bits = $308.00 (12%)
        */
        if (bits < 1500)
        {
            return (double)bits / 100 * 1.4;
        }
        if (bits < 5000)
        {
            return (double)bits / 1500 * 19.95;
        }
        if (bits < 10000)
        {
            return (double)bits / 5000 * 64.40;
        }
        if (bits < 25000)
        {
            return (double)bits / 10000 * 126;
        }
        return (double)bits / 25000 * 308;
    }
}

public class ChatReply
{
    /// <summary>Property representing the display name of the responded to message</summary>
    public string ParentDisplayName { get; internal set; }

    /// <summary>Property representing the message contents of the responded to message</summary>
    public string ParentMsgBody { get; internal set; }

    /// <summary>Property representing the id of the responded to message</summary>
    public string ParentMsgId { get; internal set; }

    /// <summary>Property representing the user id of the sender of the responded to message</summary>
    public string ParentUserId { get; internal set; }

    /// <summary>Property representing the user login of the sender of the responded to message</summary>
    public string ParentUserLogin { get; internal set; }
}

public enum Noisy
{
    NotSet,
    True,
    False
}

public static class Tags
{
    public const string Badges = "badges";
    public const string BadgeInfo = "badge-info";
    public const string BanDuration = "ban-duration";
    public const string BanReason = "ban-reason";
    public const string BroadcasterLang = "broadcaster-lang";
    public const string Bits = "bits";
    public const string Color = "color";
    public const string CustomRewardId = "custom-reward-id";
    public const string DisplayName = "display-name";
    public const string Emotes = "emotes";
    public const string EmoteOnly = "emote-only";
    public const string EmotesSets = "emote-sets";
    public const string FirstMessage = "first-msg";
    public const string Flags = "flags";
    public const string FollowersOnly = "followers-only";
    public const string Id = "id";
    public const string Login = "login";
    public const string Mercury = "mercury";
    public const string MessageId = "message-id";
    public const string Mod = "mod";
    public const string MsgId = "msg-id";   // Valid values: sub, resub, subgift, anonsubgift, submysterygift, giftpaidupgrade, rewardgift, 
                                            // anongiftpaidupgrade, raid, unraid, ritual, bitsbadgetier, announcement
    public const string MsgParamColor = "msg-param-color"; // Sent only on announcement
    public const string MsgParamDisplayname = "msg-param-displayName";                      // Sent only on raid
    public const string MsgParamLogin = "msg-param-login";                                  // Sent only on raid
    public const string MsgParamCumulativeMonths = "msg-param-cumulative-months";           // Sent only on sub, resub
    public const string MsgParamMonths = "msg-param-months";                                // Sent only on subgift, anonsubgift
    public const string MsgParamPromoGiftTotal = "msg-param-promo-gift-total";              // Sent only on anongiftpaidupgrade, giftpaidupgrade
    public const string MsgParamPromoName = "msg-param-promo-name";                         // Sent only on anongiftpaidupgrade, giftpaidupgrade
    public const string MsgParamShouldShareStreak = "msg-param-should-share-streak";        // Sent only on sub, resub
    public const string MsgParamStreakMonths = "msg-param-streak-months";                   // Sent only on sub, resub
    public const string MsgParamSubPlan = "msg-param-sub-plan";                             // Sent only on sub, resub, subgift, anonsubgift
    public const string MsgParamSubPlanName = "msg-param-sub-plan-name";                    // Sent only on sub, resub, subgift, anonsubgift
    public const string MsgParamViewerCount = "msg-param-viewerCount";                      // Sent only on raid
    public const string MsgParamRecipientDisplayname = "msg-param-recipient-display-name";  // Sent only on subgift, anonsubgift
    public const string MsgParamRecipientId = "msg-param-recipient-id";                     // Sent only on subgift, anonsubgift
    public const string MsgParamRecipientUsername = "msg-param-recipient-user-name";        // Sent only on subgift, anonsubgift
    public const string MsgParamRitualName = "msg-param-ritual-name";                       // Sent only on ritual
    public const string MsgParamMassGiftCount = "msg-param-mass-gift-count";
    public const string MsgParamSenderCount = "msg-param-sender-count";
    public const string MsgParamSenderLogin = "msg-param-sender-login";                     // Sent only on giftpaidupgrade
    public const string MsgParamSenderName = "msg-param-sender-name";                       // Sent only on giftpaidupgrade
    public const string MsgParamThreshold = "msg-param-threshold";                          // Sent only on bitsbadgetier
    public const string Noisy = "noisy";
    public const string ReplyParentDisplayName = "reply-parent-display-name";               // Sent only on replies
    public const string ReplyParentMsgBody = "reply-parent-msg-body";                       // Sent only on replies
    public const string ReplyParentMsgId = "reply-parent-msg-id";                           // Sent only on replies
    public const string ReplyParentUserId = "reply-parent-user-id";                         // Sent only on replies
    public const string ReplyParentUserLogin = "reply-parent-user-login";                   // Sent only on replies
    public const string Rituals = "rituals";
    public const string RoomId = "room-id";
    public const string R9K = "r9k";
    public const string Slow = "slow";
    public const string Subscriber = "subscriber";      // Deprecated, use badges instead
    public const string SubsOnly = "subs-only";
    public const string SystemMsg = "system-msg";
    public const string ThreadId = "thread-id";
    public const string TmiSentTs = "tmi-sent-ts";
    public const string Turbo = "turbo";                // Deprecated, use badges instead
    public const string UserId = "user-id";
    public const string UserType = "user-type";         // Deprecated, use badges instead
    public const string MsgParamMultiMonthGiftDuration = "msg-param-gift-months";             // Sent only on subgift, anonsubgift
    public const string TargetUserId = "target-user-id";
}

public class IrcMessage
{
    /// <summary>
    /// The channel the message was sent in
    /// </summary>
    public string Channel => Params.StartsWith("#") ? Params.Remove(0, 1) : Params;

    public string Params => _parameters != null && _parameters.Length > 0 ? _parameters[0] : "";

    /// <summary>
    /// Message itself
    /// </summary>
    public string Message => Trailing;

    public string Trailing => _parameters != null && _parameters.Length > 1 ? _parameters[_parameters.Length - 1] : "";

    /// <summary>
    /// Command parameters
    /// </summary>
    private readonly string[] _parameters;

    /// <summary>
    /// The user whose message it is
    /// </summary>
    public readonly string User;

    /// <summary>
    /// Hostmask of the user
    /// </summary>
    public readonly string Hostmask;

    /// <summary>
    /// Raw Command
    /// </summary>
    public readonly IrcCommand2 Command;

    /// <summary>
    /// IRCv3 tags
    /// </summary>
    public readonly Dictionary<string, string> Tags;

    /// <summary>
    /// Create an INCOMPLETE IrcMessage only carrying username
    /// </summary>
    /// <param name="user"></param>
    public IrcMessage(string user)
    {
        _parameters = null;
        User = user;
        Hostmask = null;
        Command = IrcCommand2.Unknown;
        Tags = null;
    }

    /// <summary>
    /// Create an IrcMessage
    /// </summary>
    /// <param name="command">IRC Command</param>
    /// <param name="parameters">Command params</param>
    /// <param name="hostmask">User</param>
    /// <param name="tags">IRCv3 tags</param>
    public IrcMessage(
        IrcCommand2 command,
        string[] parameters,
        string hostmask,
        Dictionary<string, string> tags = null)
    {
        var idx = hostmask.IndexOf('!');
        User = idx != -1 ? hostmask.Substring(0, idx) : hostmask;
        Hostmask = hostmask;
        _parameters = parameters;
        Command = command;
        Tags = tags;

        if (command == IrcCommand2.RPL_353)
        {
            if (Params.Length > 0 && Params.Contains("#"))
            {
                _parameters[0] = $"#{_parameters[0].Split('#')[1]}";
            }
        }
    }

    public new string ToString()
    {
        var raw = new StringBuilder(32);
        if (Tags != null)
        {
            var tags = new string[Tags.Count];
            var i = 0;
            foreach (var tag in Tags)
            {
                tags[i] = tag.Key + "=" + tag.Value;
                ++i;
            }

            if (tags.Length > 0)
            {
                raw.Append("@").Append(string.Join(";", tags)).Append(" ");
            }
        }

        if (!string.IsNullOrEmpty(Hostmask))
        {
            raw.Append(":").Append(Hostmask).Append(" ");
        }

        raw.Append(Command.ToString().ToUpper().Replace("RPL_", ""));
        if (_parameters.Length <= 0)
            return raw.ToString();

        if (_parameters[0] != null && _parameters[0].Length > 0)
        {
            raw.Append(" ").Append(_parameters[0]);
        }

        if (_parameters.Length > 1 && _parameters[1] != null && _parameters[1].Length > 0)
        {
            raw.Append(" :").Append(_parameters[1]);
        }

        return raw.ToString();
    }
}

public class OnIncorrectLoginArgs : EventArgs
{
    /// <summary>
    /// Property representing exception object.
    /// </summary>
    public ErrorLoggingInException Exception;
}

public class ErrorLoggingInException : Exception
{
    /// <summary>
    /// Exception representing username associated with bad login.
    /// </summary>
    /// <value>The username.</value>
    public string Username { get; protected set; }

    /// <summary>
    /// Exception construtor.
    /// </summary>
    /// <param name="ircData">The irc data.</param>
    /// <param name="twitchUsername">The twitch username.</param>
    /// <inheritdoc />
    public ErrorLoggingInException(string ircData, string twitchUsername)
        : base(ircData)
    {
        Username = twitchUsername;
    }
}

public static class MsgIds
{
    public const string AlreadyBanned = "already_banned";
    public const string AlreadyEmotesOff = "already_emotes_off";
    public const string AlreadyEmotesOn = "already_emotes_on";
    public const string AlreadyR9KOff = "already_r9k_off";
    public const string AlreadyR9KOn = "already_r9k_on";
    public const string AlreadySubsOff = "already_subs_off";
    public const string AlreadySubsOn = "already_subs_on";
    public const string Announcement = "announcement";
    public const string BadUnbanNoBan = "bad_unban_no_ban";
    public const string BanSuccess = "ban_success";
    public const string ColorChanged = "color_changed";
    public const string EmoteOnlyOff = "emote_only_off";
    public const string EmoteOnlyOn = "emote_only_on";
    public const string HighlightedMessage = "highlighted-message";
    public const string ModeratorsReceived = "room_mods";
    public const string NoMods = "no_mods";
    public const string NoVIPs = "no_vips";
    public const string MsgBannedEmailAlias = "msg_banned_email_alias";
    public const string MsgChannelSuspended = "msg_channel_suspended";
    public const string MsgRequiresVerifiedPhoneNumber = "msg_requires_verified_phone_number";
    public const string MsgVerifiedEmail = "msg_verified_email";
    public const string MsgRateLimit = "msg_ratelimit";
    public const string MsgDuplicate = "msg_duplicate";
    public const string MsgR9k = "msg_r9k";
    public const string MsgFollowersOnly = "msg_followersonly";
    public const string MsgSubsOnly = "msg_subsonly";
    public const string MsgEmoteOnly = "msg_emoteonly";
    public const string MsgSuspended = "msg_suspended";
    public const string MsgBanned = "msg_banned";
    public const string MsgSlowMode = "msg_slowmode";
    public const string NoPermission = "no_permission";
    public const string PrimePaidUprade = "primepaidupgrade";
    public const string Raid = "raid";
    public const string RaidErrorSelf = "raid_error_self";
    public const string RaidNoticeMature = "raid_notice_mature";
    public const string ReSubscription = "resub";
    public const string R9KOff = "r9k_off";
    public const string R9KOn = "r9k_on";
    public const string SubGift = "subgift";
    public const string CommunitySubscription = "submysterygift";
    public const string ContinuedGiftedSubscription = "giftpaidupgrade";
    public const string Subscription = "sub";
    public const string SubsOff = "subs_off";
    public const string SubsOn = "subs_on";
    public const string TimeoutSuccess = "timeout_success";
    public const string UnbanSuccess = "unban_success";
    public const string UnrecognizedCmd = "unrecognized_cmd";
    public const string UserIntro = "user-intro";
    public const string VIPsSuccess = "vips_success";
    public const string SkipSubsModeMessage = "skip-subs-mode-message";
}
