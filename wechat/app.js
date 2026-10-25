const config = require('./config');

App({
  globalData: {
    userId: null,
    authError: null
  },

  onLaunch() {
    this.login();
  },

  /**
   * Exchanges a WeChat login code for an application user id.
   *
   * The backend `/auth/wechat` endpoint is part of milestone 2. Until it
   * exists, callers fall back to the seeded demo account via resolveUserId().
   */
  login() {
    if (this.globalData.userId) {
      return Promise.resolve(this.globalData.userId);
    }
    if (this._loginPromise) {
      return this._loginPromise;
    }

    this._loginPromise = new Promise((resolve) => {
      wx.login({
        success: (result) => {
          if (!result.code) {
            this.globalData.authError = 'wx.login returned no code';
            resolve(null);
            return;
          }

          wx.request({
            url: `${config.baseUrl}/auth/wechat`,
            method: 'POST',
            data: { code: result.code },
            success: (response) => {
              const userId = response.data && response.data.user_id;
              if (response.statusCode >= 200 && response.statusCode < 300 && userId) {
                this.globalData.userId = userId;
                this.globalData.authError = null;
                resolve(userId);
              } else {
                this.globalData.authError =
                  (response.data && response.data.error) ||
                  `auth request failed with HTTP ${response.statusCode}`;
                resolve(null);
              }
            },
            fail: (error) => {
              this.globalData.authError = error.errMsg || 'auth request failed';
              resolve(null);
            }
          });
        },
        fail: (error) => {
          this.globalData.authError = error.errMsg || 'wx.login failed';
          resolve(null);
        }
      });
    });

    this._loginPromise.then(() => {
      this._loginPromise = null;
    });

    return this._loginPromise;
  },

  resolveUserId() {
    return this.globalData.userId || config.defaultUserId;
  }
});
