#import <UIKit/UIKit.h>
#import <WebKit/WebKit.h>
#import <arpa/inet.h>

static NSString * const A4NLastURLKey = @"A4NRemoteLastURL";
static NSString * const A4NDefaultURL = @"https://api.imggb.top/a4n/";

static BOOL A4NPrivateIPv4(NSString *host) {
    struct in_addr address;
    if (inet_pton(AF_INET, host.UTF8String, &address) != 1) return NO;
    uint32_t value = ntohl(address.s_addr);
    return ((value >> 24) == 10)
        || (value >= 0xAC100000 && value <= 0xAC1FFFFF)
        || (value >= 0xC0A80000 && value <= 0xC0A8FFFF)
        || (value >= 0xA9FE0000 && value <= 0xA9FEFFFF)
        || ((value >> 24) == 127);
}

static BOOL A4NPrivateIPv6(NSString *host) {
    struct in6_addr address;
    if (inet_pton(AF_INET6, host.UTF8String, &address) != 1) return NO;
    const unsigned char *bytes = address.__u6_addr.__u6_addr8;
    BOOL loopback = YES;
    for (NSUInteger index = 0; index < 15; index += 1) {
        if (bytes[index] != 0) { loopback = NO; break; }
    }
    loopback = loopback && bytes[15] == 1;
    BOOL uniqueLocal = (bytes[0] & 0xfe) == 0xfc;
    BOOL linkLocal = bytes[0] == 0xfe && (bytes[1] & 0xc0) == 0x80;
    return loopback || uniqueLocal || linkLocal;
}

static NSURL *A4NValidatedURL(NSString *value) {
    NSString *trimmed = [value stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
    if (trimmed.length == 0) return nil;
    NSURL *url = [NSURL URLWithString:trimmed];
    if (url == nil || url.host.length == 0 || url.user != nil || url.password != nil) return nil;

    NSString *scheme = url.scheme.lowercaseString;
    if ([scheme isEqualToString:@"https"]) return url;
    if (![scheme isEqualToString:@"http"]) return nil;

    NSString *host = url.host.lowercaseString;
    if ([host isEqualToString:@"localhost"] || [host hasSuffix:@".local"]
        || A4NPrivateIPv4(host) || A4NPrivateIPv6(host)) return url;
    return nil;
}

static NSString *A4NOrigin(NSURL *url) {
    NSURLComponents *components = [NSURLComponents componentsWithURL:url resolvingAgainstBaseURL:NO];
    NSString *scheme = components.scheme.lowercaseString ?: @"";
    NSString *host = components.host.lowercaseString ?: @"";
    if ([host containsString:@":"]) host = [NSString stringWithFormat:@"[%@]", host];
    BOOL defaultPort = ([scheme isEqualToString:@"https"] && components.port.integerValue == 443)
        || ([scheme isEqualToString:@"http"] && components.port.integerValue == 80);
    NSString *port = components.port == nil || defaultPort ? @"" : [NSString stringWithFormat:@":%@", components.port];
    return [NSString stringWithFormat:@"%@://%@%@", scheme, host, port];
}

@interface A4NWebViewController : UIViewController <WKNavigationDelegate, WKUIDelegate, WKDownloadDelegate>
@property(nonatomic, strong) WKWebView *webView;
@property(nonatomic, strong) UIRefreshControl *refreshControl;
@property(nonatomic, strong) UIActivityIndicatorView *activity;
@property(nonatomic, strong) UIView *errorView;
@property(nonatomic, strong) NSURL *initialURL;
@property(nonatomic, copy) NSString *allowedOrigin;
@property(nonatomic, copy) void (^onChangeConnection)(void);
@end

@implementation A4NWebViewController

- (void)viewDidLoad {
    [super viewDidLoad];
    self.view.backgroundColor = UIColor.systemBackgroundColor;
    self.title = @"A4N";
    self.allowedOrigin = A4NOrigin(self.initialURL);
    self.navigationItem.rightBarButtonItem = [[UIBarButtonItem alloc] initWithTitle:@"服务器" style:UIBarButtonItemStylePlain target:self action:@selector(changeConnection)];

    WKWebViewConfiguration *configuration = [[WKWebViewConfiguration alloc] init];
    configuration.allowsInlineMediaPlayback = YES;
    configuration.defaultWebpagePreferences.preferredContentMode = WKContentModeMobile;
    self.webView = [[WKWebView alloc] initWithFrame:CGRectZero configuration:configuration];
    self.webView.navigationDelegate = self;
    self.webView.UIDelegate = self;
    self.webView.allowsBackForwardNavigationGestures = YES;
    self.webView.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:self.webView];
    [NSLayoutConstraint activateConstraints:@[
        [self.webView.leadingAnchor constraintEqualToAnchor:self.view.leadingAnchor],
        [self.webView.trailingAnchor constraintEqualToAnchor:self.view.trailingAnchor],
        [self.webView.topAnchor constraintEqualToAnchor:self.view.topAnchor],
        [self.webView.bottomAnchor constraintEqualToAnchor:self.view.bottomAnchor],
    ]];

    self.refreshControl = [[UIRefreshControl alloc] init];
    [self.refreshControl addTarget:self action:@selector(refresh) forControlEvents:UIControlEventValueChanged];
    [self.webView.scrollView addSubview:self.refreshControl];

    self.activity = [[UIActivityIndicatorView alloc] initWithActivityIndicatorStyle:UIActivityIndicatorViewStyleMedium];
    self.activity.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:self.activity];
    [NSLayoutConstraint activateConstraints:@[
        [self.activity.centerXAnchor constraintEqualToAnchor:self.view.centerXAnchor],
        [self.activity.centerYAnchor constraintEqualToAnchor:self.view.centerYAnchor],
    ]];
    [self.webView loadRequest:[NSURLRequest requestWithURL:self.initialURL cachePolicy:NSURLRequestUseProtocolCachePolicy timeoutInterval:30.0]];
}

- (void)refresh { [self.webView reload]; }

- (void)changeConnection {
    if (self.onChangeConnection != nil) self.onChangeConnection();
}

- (void)showError {
    if (self.errorView != nil) return;
    UIView *panel = [[UIView alloc] initWithFrame:CGRectZero];
    panel.backgroundColor = UIColor.systemBackgroundColor;
    panel.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:panel];
    UILabel *title = [[UILabel alloc] initWithFrame:CGRectZero];
    title.text = @"无法连接 A4N";
    title.font = [UIFont preferredFontForTextStyle:UIFontTextStyleTitle2];
    title.textAlignment = NSTextAlignmentCenter;
    title.translatesAutoresizingMaskIntoConstraints = NO;
    [panel addSubview:title];
    UILabel *detail = [[UILabel alloc] initWithFrame:CGRectZero];
    detail.text = @"请检查服务器地址和网络，然后重试。";
    detail.textColor = UIColor.secondaryLabelColor;
    detail.font = [UIFont preferredFontForTextStyle:UIFontTextStyleBody];
    detail.numberOfLines = 0;
    detail.textAlignment = NSTextAlignmentCenter;
    detail.translatesAutoresizingMaskIntoConstraints = NO;
    [panel addSubview:detail];
    UIButton *retry = [UIButton buttonWithType:UIButtonTypeSystem];
    [retry setTitle:@"重新连接" forState:UIControlStateNormal];
    retry.titleLabel.font = [UIFont preferredFontForTextStyle:UIFontTextStyleHeadline];
    retry.accessibilityLabel = @"重新连接 A4N";
    retry.translatesAutoresizingMaskIntoConstraints = NO;
    [retry addTarget:self action:@selector(retry) forControlEvents:UIControlEventTouchUpInside];
    [panel addSubview:retry];
    [NSLayoutConstraint activateConstraints:@[
        [panel.leadingAnchor constraintEqualToAnchor:self.view.leadingAnchor], [panel.trailingAnchor constraintEqualToAnchor:self.view.trailingAnchor],
        [panel.topAnchor constraintEqualToAnchor:self.view.topAnchor], [panel.bottomAnchor constraintEqualToAnchor:self.view.bottomAnchor],
        [title.centerXAnchor constraintEqualToAnchor:panel.centerXAnchor], [title.centerYAnchor constraintEqualToAnchor:panel.centerYAnchor constant:-42],
        [detail.leadingAnchor constraintEqualToAnchor:panel.leadingAnchor constant:28], [detail.trailingAnchor constraintEqualToAnchor:panel.trailingAnchor constant:-28],
        [detail.topAnchor constraintEqualToAnchor:title.bottomAnchor constant:12],
        [retry.centerXAnchor constraintEqualToAnchor:panel.centerXAnchor], [retry.topAnchor constraintEqualToAnchor:detail.bottomAnchor constant:20],
        [retry.widthAnchor constraintGreaterThanOrEqualToConstant:120], [retry.heightAnchor constraintGreaterThanOrEqualToConstant:44],
    ]];
    self.errorView = panel;
}

- (void)retry {
    [self.errorView removeFromSuperview];
    self.errorView = nil;
    [self.webView loadRequest:[NSURLRequest requestWithURL:self.initialURL cachePolicy:NSURLRequestUseProtocolCachePolicy timeoutInterval:30.0]];
}

- (void)webView:(WKWebView *)webView didStartProvisionalNavigation:(WKNavigation *)navigation {
    [self.activity startAnimating];
    (void)webView; (void)navigation;
}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation {
    [self.activity stopAnimating]; [self.refreshControl endRefreshing];
    [self.errorView removeFromSuperview]; self.errorView = nil;
    (void)webView; (void)navigation;
}

- (void)webView:(WKWebView *)webView didFailProvisionalNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    [self.activity stopAnimating]; [self.refreshControl endRefreshing];
    if (error.code != NSURLErrorCancelled) [self showError];
    (void)webView; (void)navigation;
}

- (void)webView:(WKWebView *)webView didFailNavigation:(WKNavigation *)navigation withError:(NSError *)error {
    [self.activity stopAnimating]; [self.refreshControl endRefreshing];
    if (error.code != NSURLErrorCancelled) [self showError];
    (void)webView; (void)navigation;
}

- (void)webView:(WKWebView *)webView decidePolicyForNavigationAction:(WKNavigationAction *)action decisionHandler:(void (^)(WKNavigationActionPolicy))decisionHandler {
    NSURL *url = action.request.URL;
    NSString *scheme = url.scheme.lowercaseString;
    BOOL webURL = [scheme isEqualToString:@"https"] || [scheme isEqualToString:@"http"];
    BOOL sameOrigin = url != nil && [A4NOrigin(url) isEqualToString:self.allowedOrigin];
    if (action.shouldPerformDownload) {
        NSURL *sourceURL = action.sourceFrame.request.URL ?: action.request.mainDocumentURL;
        BOOL trustedSource = sourceURL != nil && [A4NOrigin(sourceURL) isEqualToString:self.allowedOrigin];
        BOOL allowedTarget = (webURL && sameOrigin) || [scheme isEqualToString:@"blob"];
        decisionHandler(trustedSource && allowedTarget ? WKNavigationActionPolicyDownload : WKNavigationActionPolicyCancel);
        return;
    }
    if (webURL && sameOrigin) { decisionHandler(WKNavigationActionPolicyAllow); return; }
    BOOL topLevel = action.targetFrame == nil || action.targetFrame.isMainFrame;
    if (webURL && topLevel) [[UIApplication sharedApplication] openURL:url options:@{} completionHandler:nil];
    decisionHandler(WKNavigationActionPolicyCancel);
    (void)webView;
}

- (void)webView:(WKWebView *)webView decidePolicyForNavigationResponse:(WKNavigationResponse *)response decisionHandler:(void (^)(WKNavigationResponsePolicy))decisionHandler {
    decisionHandler(response.canShowMIMEType ? WKNavigationResponsePolicyAllow : WKNavigationResponsePolicyDownload);
    (void)webView;
}

- (void)webView:(WKWebView *)webView navigationAction:(WKNavigationAction *)navigationAction didBecomeDownload:(WKDownload *)download {
    download.delegate = self;
    (void)webView; (void)navigationAction;
}

- (void)webView:(WKWebView *)webView navigationResponse:(WKNavigationResponse *)navigationResponse didBecomeDownload:(WKDownload *)download {
    download.delegate = self;
    (void)webView; (void)navigationResponse;
}

- (void)download:(WKDownload *)download decideDestinationUsingResponse:(NSURLResponse *)response suggestedFilename:(NSString *)suggestedFilename completionHandler:(void (^)(NSURL *destination))completionHandler {
    NSURL *documents = [NSFileManager.defaultManager URLsForDirectory:NSDocumentDirectory inDomains:NSUserDomainMask].firstObject;
    NSString *fileName = suggestedFilename.lastPathComponent.length == 0 ? @"A4N-download" : suggestedFilename.lastPathComponent;
    if ([fileName isEqualToString:@"."] || [fileName isEqualToString:@".."]) fileName = @"A4N-download";
    NSURL *destination = [documents URLByAppendingPathComponent:fileName];
    if ([NSFileManager.defaultManager fileExistsAtPath:destination.path]) {
        NSString *extension = fileName.pathExtension;
        NSString *base = fileName.stringByDeletingPathExtension;
        NSString *suffix = NSUUID.UUID.UUIDString;
        fileName = extension.length == 0
            ? [NSString stringWithFormat:@"%@-%@", base, suffix]
            : [NSString stringWithFormat:@"%@-%@.%@", base, suffix, extension];
        destination = [documents URLByAppendingPathComponent:fileName];
    }
    completionHandler(destination);
    (void)download; (void)response;
}

- (void)download:(WKDownload *)download didFailWithError:(NSError *)error resumeData:(NSData *)resumeData {
    UIAlertController *alert = [UIAlertController alertControllerWithTitle:@"下载失败" message:error.localizedDescription preferredStyle:UIAlertControllerStyleAlert];
    [alert addAction:[UIAlertAction actionWithTitle:@"确定" style:UIAlertActionStyleDefault handler:nil]];
    [self presentViewController:alert animated:YES completion:nil];
    (void)download; (void)resumeData;
}

- (WKWebView *)webView:(WKWebView *)webView createWebViewWithConfiguration:(WKWebViewConfiguration *)configuration forNavigationAction:(WKNavigationAction *)navigationAction windowFeatures:(WKWindowFeatures *)windowFeatures {
    NSURL *url = navigationAction.request.URL;
    NSString *scheme = url.scheme.lowercaseString;
    BOOL webURL = [scheme isEqualToString:@"https"] || [scheme isEqualToString:@"http"];
    if (webURL && url != nil && [A4NOrigin(url) isEqualToString:self.allowedOrigin]) [webView loadRequest:navigationAction.request];
    else if (webURL) [[UIApplication sharedApplication] openURL:url options:@{} completionHandler:nil];
    (void)configuration; (void)windowFeatures;
    return nil;
}

- (void)webView:(WKWebView *)webView runJavaScriptTextInputPanelWithPrompt:(NSString *)prompt defaultText:(NSString *)defaultText initiatedByFrame:(WKFrameInfo *)frame completionHandler:(void (^)(NSString *result))completionHandler {
    UIAlertController *alert = [UIAlertController alertControllerWithTitle:@"A4N" message:prompt preferredStyle:UIAlertControllerStyleAlert];
    [alert addTextFieldWithConfigurationHandler:^(UITextField *field) { field.text = defaultText; }];
    [alert addAction:[UIAlertAction actionWithTitle:@"取消" style:UIAlertActionStyleCancel handler:^(__unused UIAlertAction *action) { completionHandler(nil); }]];
    [alert addAction:[UIAlertAction actionWithTitle:@"确定" style:UIAlertActionStyleDefault handler:^(__unused UIAlertAction *action) { completionHandler(alert.textFields.firstObject.text); }]];
    [self presentViewController:alert animated:YES completion:nil];
    (void)webView; (void)frame;
}

@end

@interface A4NConnectViewController : UIViewController <UITextFieldDelegate, UIGestureRecognizerDelegate>
@property(nonatomic, strong) UITextField *urlField;
@property(nonatomic, copy) void (^onConnect)(NSURL *url);
@end

@implementation A4NConnectViewController

- (void)viewDidLoad {
    [super viewDidLoad];
    self.view.backgroundColor = UIColor.systemBackgroundColor;
    self.title = @"连接 A4N";
    UILabel *title = [[UILabel alloc] initWithFrame:CGRectZero];
    title.text = @"连接 A4N 服务器";
    title.font = [UIFont preferredFontForTextStyle:UIFontTextStyleLargeTitle];
    title.translatesAutoresizingMaskIntoConstraints = NO;
    UILabel *detail = [[UILabel alloc] initWithFrame:CGRectZero];
    detail.text = @"输入服务器 Web 入口，例如 https://a4n.example.com/a4n/；不要填写 /api 地址。";
    detail.textColor = UIColor.secondaryLabelColor;
    detail.numberOfLines = 0;
    detail.translatesAutoresizingMaskIntoConstraints = NO;
    self.urlField = [[UITextField alloc] initWithFrame:CGRectZero];
    self.urlField.placeholder = @"https://…";
    self.urlField.borderStyle = UITextBorderStyleRoundedRect;
    self.urlField.keyboardType = UIKeyboardTypeURL;
    self.urlField.textContentType = UITextContentTypeURL;
    self.urlField.accessibilityLabel = @"服务器网址";
    self.urlField.delegate = self;
    self.urlField.returnKeyType = UIReturnKeyGo;
    self.urlField.autocapitalizationType = UITextAutocapitalizationTypeNone;
    self.urlField.autocorrectionType = UITextAutocorrectionTypeNo;
    self.urlField.clearButtonMode = UITextFieldViewModeWhileEditing;
    UIToolbar *keyboardToolbar = [[UIToolbar alloc] init];
    keyboardToolbar.items = @[
        [[UIBarButtonItem alloc] initWithBarButtonSystemItem:UIBarButtonSystemItemFlexibleSpace target:nil action:nil],
        [[UIBarButtonItem alloc] initWithTitle:@"完成" style:UIBarButtonItemStyleDone target:self action:@selector(dismissKeyboard)],
    ];
    [keyboardToolbar sizeToFit];
    self.urlField.inputAccessoryView = keyboardToolbar;
    self.urlField.translatesAutoresizingMaskIntoConstraints = NO;
    UIButton *connect = [UIButton buttonWithType:UIButtonTypeSystem];
    [connect setTitle:@"打开服务器" forState:UIControlStateNormal];
    connect.titleLabel.font = [UIFont preferredFontForTextStyle:UIFontTextStyleHeadline];
    connect.accessibilityLabel = @"打开 A4N 服务器";
    connect.translatesAutoresizingMaskIntoConstraints = NO;
    [connect addTarget:self action:@selector(connect) forControlEvents:UIControlEventTouchUpInside];
    UITapGestureRecognizer *dismissTap = [[UITapGestureRecognizer alloc] initWithTarget:self action:@selector(dismissKeyboard)];
    dismissTap.delegate = self;
    dismissTap.cancelsTouchesInView = NO;
    [self.view addGestureRecognizer:dismissTap];
    UIStackView *stack = [[UIStackView alloc] initWithArrangedSubviews:@[title, detail, self.urlField, connect]];
    stack.axis = UILayoutConstraintAxisVertical; stack.spacing = 16; stack.translatesAutoresizingMaskIntoConstraints = NO;
    [self.view addSubview:stack];
    UILayoutGuide *safe = self.view.safeAreaLayoutGuide;
    [NSLayoutConstraint activateConstraints:@[
        [stack.leadingAnchor constraintEqualToAnchor:safe.leadingAnchor constant:20], [stack.trailingAnchor constraintEqualToAnchor:safe.trailingAnchor constant:-20],
        [stack.centerYAnchor constraintEqualToAnchor:safe.centerYAnchor], [connect.heightAnchor constraintGreaterThanOrEqualToConstant:50],
    ]];
}

- (void)dismissKeyboard {
    [self.view endEditing:YES];
}

- (BOOL)gestureRecognizer:(UIGestureRecognizer *)gestureRecognizer shouldReceiveTouch:(UITouch *)touch {
    UIView *touchedView = touch.view;
    BOOL insideURLField = touchedView == self.urlField || [touchedView isDescendantOfView:self.urlField];
    (void)gestureRecognizer;
    return !insideURLField;
}

- (BOOL)textFieldShouldReturn:(UITextField *)textField {
    [self connect];
    (void)textField;
    return YES;
}

- (void)connect {
    [self dismissKeyboard];
    NSURL *url = A4NValidatedURL(self.urlField.text);
    if (url == nil) {
        UIAlertController *alert = [UIAlertController alertControllerWithTitle:@"地址不可用" message:@"公网必须使用 HTTPS；局域网 HTTP 仅支持私网地址。" preferredStyle:UIAlertControllerStyleAlert];
        [alert addAction:[UIAlertAction actionWithTitle:@"确定" style:UIAlertActionStyleDefault handler:nil]];
        [self presentViewController:alert animated:YES completion:nil];
        return;
    }
    if (self.onConnect != nil) self.onConnect(url);
}

@end

@interface A4NAppDelegate : UIResponder <UIApplicationDelegate>
@property(nonatomic, strong) UIWindow *window;
@end

@implementation A4NAppDelegate

- (BOOL)application:(UIApplication *)application didFinishLaunchingWithOptions:(NSDictionary *)launchOptions {
    self.window = [[UIWindow alloc] initWithFrame:UIScreen.mainScreen.bounds];
    NSURL *saved = A4NValidatedURL([NSUserDefaults.standardUserDefaults stringForKey:A4NLastURLKey]);
    if (saved == nil) saved = A4NValidatedURL(A4NDefaultURL);
    [self showURL:saved];
    [self.window makeKeyAndVisible];
    (void)application; (void)launchOptions;
    return YES;
}

- (void)showURL:(NSURL *)url {
    if (url == nil) {
        A4NConnectViewController *controller = [[A4NConnectViewController alloc] init];
        __weak typeof(self) weakSelf = self;
        controller.onConnect = ^(NSURL *selectedURL) {
            [NSUserDefaults.standardUserDefaults setObject:selectedURL.absoluteString forKey:A4NLastURLKey];
            [weakSelf showURL:selectedURL];
        };
        self.window.rootViewController = [[UINavigationController alloc] initWithRootViewController:controller];
        return;
    }
    A4NWebViewController *controller = [[A4NWebViewController alloc] init];
    controller.initialURL = url;
    __weak typeof(self) weakSelf = self;
    controller.onChangeConnection = ^{
        [NSUserDefaults.standardUserDefaults removeObjectForKey:A4NLastURLKey];
        [weakSelf showURL:nil];
    };
    self.window.rootViewController = [[UINavigationController alloc] initWithRootViewController:controller];
}

@end

int main(int argc, char *argv[]) {
    @autoreleasepool {
        return UIApplicationMain(argc, argv, nil, NSStringFromClass([A4NAppDelegate class]));
    }
}
